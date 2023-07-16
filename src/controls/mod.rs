use std::borrow::Cow;
use std::time::Duration;
use adw::gio::File;
use adw::prelude::*;
use ContentFit::Contain;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::ClockTime;
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{Continue, ElementExt, ElementExtManual, ObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, ContentFit, IconLookupFlags, IconTheme, Inhibit, Label, Picture, ProgressBar, Scale, ScrollType, TextDirection};
use gtk::Orientation::{Horizontal, Vertical};
use log::warn;
use mpris_player::{Metadata, PlaybackStatus};
use util::format;
use crate::collection::model::Collection;
use crate::collection::song::{get_current_song, join_path, Song, WithCover};
use crate::collection::song::WithPath;
use crate::common::{BoldLabelBuilder, EllipsizedLabelBuilder, util};
use crate::common::util::or_none;
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::controls::mpris::mpris_player;
use crate::controls::playbin::{PLAYBIN, Playbin, URI};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

mod playbin;
mod mpris;

trait Playable {
    fn change_state(&self, icon: &str, tooltip: &str);
    fn play(&self);
    fn pause(&self);
}

impl Playable for Button {
    fn change_state(&self, icon: &str, tooltip: &str) {
        self.set_icon_name(icon);
        self.set_tooltip_text(Some(tooltip));
    }
    fn play(&self) {
        self.change_state("media-playback-start", "Play");
    }
    fn pause(&self) {
        self.change_state("media-playback-pause", "Pause");
    }
}

pub fn media_controls() -> Wrapper {
    let mpris_player = mpris_player();
    let now_playing = gtk::Box::builder().orientation(Horizontal)
        .margin_start(8).margin_end(8).margin_top(8).margin_bottom(8).build();
    let song_info = gtk::Box::builder().orientation(Vertical).build();
    let unknown_album_file = IconTheme::for_display(&now_playing.display())
        .lookup_icon("audio-x-generic", &[], 128, 1, TextDirection::None, IconLookupFlags::empty()).file().unwrap();
    let picture_file = if let Ok((song, _, collection)) = get_current_song(&mut get_connection()) {
        Cow::Owned(File::for_path((&song, &collection).path()))
    } else {
        Cow::Borrowed(&unknown_album_file)
    };
    let album_picture = Picture::builder().content_fit(Contain).file(&*picture_file).build();
    now_playing.append(&album_picture);
    now_playing.append(&song_info);
    let play_pause = Button::builder().width_request(40).build();
    play_pause.play();
    now_playing.append(&play_pause);
    let song_label = Label::builder().margin_ellipsized(4).bold().build();
    let artist_label = Label::builder().margin_ellipsized(4).build();
    song_info.append(&song_label);
    song_info.append(&artist_label);
    let time_box = gtk::Box::builder().spacing(4).margin_start(4).build();
    let position_label = Label::new(Some(&format(0)));
    time_box.append(&position_label);
    time_box.append(&Label::new(Some("/")));
    let duration_label = Label::new(Some(&format(0)));
    time_box.append(&duration_label);
    song_info.append(&time_box);
    let scale = Scale::builder().hexpand(true).build();
    scale.set_range(0.0, 1.0);
    let now_playing_and_progress = gtk::Box::builder().orientation(Vertical).build();
    let progress_bar = ProgressBar::new();
    progress_bar.add_css_class("osd");
    now_playing_and_progress.append(&progress_bar);
    now_playing_and_progress.append(&now_playing);
    play_pause.connect_clicked(move |play_pause| {
        if PLAYBIN.current_state() == Playing {
            PLAYBIN.set_state(Paused).unwrap();
            play_pause.play();
        } else {
            match PLAYBIN.set_state(Playing) {
                Ok(_) => { play_pause.pause(); }
                Err(error) => { warn!("error trying to play {} {}", PLAYBIN.property::<String>(URI), error); }
            }
        }
    });
    scale.connect_change_value({
        let position_label = position_label.clone();
        let progress_bar = progress_bar.clone();
        let scale = scale.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                match PLAYBIN.get_duration() {
                    None => { warn!("cannot seek when duration is not available"); }
                    Some(duration) => {
                        if let Err(error) = PLAYBIN.seek_internal(value as u64, &position_label, &progress_bar,
                            duration, &scale) {
                            warn!("error trying to seek to {} {}", value, error);
                        }
                    }
                }
            }
            Inhibit(true)
        }
    });
    let wrapper = Wrapper::new(&now_playing_and_progress);
    wrapper.connect_local(SONG_SELECTED, true, {
        let play_pause = play_pause.clone();
        move |params| {
            if let [_, current_song_path, collection_path] = &params {
                let playing = PLAYBIN.current_state() == Playing;
                PLAYBIN.set_state(Null).unwrap();
                PLAYBIN.set_uri(&join_path(&collection_path.get::<String>().unwrap(),
                    &current_song_path.get::<String>().unwrap()));
                if playing {
                    PLAYBIN.set_state(Playing).unwrap();
                } else {
                    play_pause.emit_clicked();
                }
            }
            None
        }
    });
    mpris_player.connect_play_pause({
        let play_pause = play_pause.clone();
        move || { play_pause.emit_clicked(); }
    });
    mpris_player.connect_play({
        let play_pause = play_pause.clone();
        move || { if PLAYBIN.current_state() != Playing { play_pause.emit_clicked(); } }
    });
    mpris_player.connect_pause(move || { if PLAYBIN.current_state() != Paused { play_pause.emit_clicked(); } });
    mpris_player.connect_seek({
        let position_label = position_label.clone();
        let progress_bar = progress_bar.clone();
        let scale = scale.clone();
        move |delta_micros| {
            PLAYBIN.simple_seek(Duration::from_micros(delta_micros.abs() as u64), delta_micros >= 0,
                &position_label, &progress_bar, &scale);
        }
    });
    PLAYBIN.bus().unwrap().add_watch_local({
        let scale = scale.clone();
        let wrapper = wrapper.clone();
        move |_, message| {
            match message.view() {
                StateChanged(state_changed) => {
                    match state_changed.current() {
                        Playing => {
                            mpris_player.set_playback_status(PlaybackStatus::Playing);
                            timeout_add_local(Duration::from_millis(200), {
                                let position_label = position_label.clone();
                                let scale = scale.clone();
                                let progress_bar = progress_bar.clone();
                                let mpris_player = mpris_player.clone();
                                move || {
                                    if let Some(position) = PLAYBIN.get_position() {
                                        position_label.set_label(&format(position));
                                        scale.set_value(position as f64);
                                        if let Some(duration) = PLAYBIN.get_duration() {
                                            progress_bar.set_fraction(position as f64 / duration as f64);
                                        }
                                        mpris_player.set_position(position as i64);
                                    }
                                    Continue(PLAYBIN.current_state() == Playing || PLAYBIN.pending_state() == Playing)
                                }
                            });
                        }
                        Paused => { mpris_player.set_playback_status(PlaybackStatus::Paused); }
                        _ => {}
                    }
                }
                AsyncDone(_) | DurationChanged(_) => {
                    if let Some(duration) = PLAYBIN.query_duration().map(ClockTime::nseconds) {
                        duration_label.set_label(&format(duration));
                        scale.set_range(0.0, duration as f64);
                    }
                }
                StreamStart(_) => {
                    let uri = &PLAYBIN.property::<String>("current-uri")[5.. /* remove "file:" */];
                    get_connection().transaction(|connection| {
                        let (collection, song) = collections.inner_join(songs)
                            .filter(path.concat("/").concat(song_path).eq(uri))
                            .get_result::<(Collection, Song)>(connection)?;
                        update(config).set(current_song_id.eq(song.id)).execute(connection)?;
                        artist_label.set_label(or_none(&song.artist));
                        let title = song.title_str().to_owned();
                        song_label.set_label(&title);
                        let cover = (&song, &collection).path().cover();
                        let art_url = if cover.exists() {
                            album_picture.set_filename(Some(&cover));
                            cover.to_str().map(|it| { format!("file:{}", it) })
                        } else {
                            album_picture.set_file(Some(&unknown_album_file));
                            None
                        };
                        wrapper.emit_by_name::<()>(STREAM_STARTED, &[&song.id]);
                        mpris_player.set_metadata(Metadata {
                            length: Some(song.duration),
                            art_url,
                            album: song.album,
                            album_artist: song.album_artist.map(|it| { vec![it] }),
                            artist: song.artist.map(|it| { vec![it] }),
                            composer: None,
                            disc_number: None,
                            genre: song.genre.map(|it| { vec![it] }),
                            title: Some(title),
                            track_number: song.track_number,
                            url: Some(uri.to_string()),
                        });
                        anyhow::Ok(())
                    }).unwrap();
                }
                _ => {}
            }
            Continue(true)
        }
    }).unwrap();
    wrapper
}
