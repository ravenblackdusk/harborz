use std::time::Duration;
use adw::gdk::pango::{AttrInt, AttrList, Weight};
use adw::prelude::*;
use ContentFit::Contain;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::ClockTime;
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{Continue, ElementExt, ElementExtManual, ObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, ContentFit, Grid, IconLookupFlags, IconTheme, Inhibit, Label, Picture, Scale, ScrollType, TextDirection};
use gtk::gdk::SeatCapabilities;
use gtk::Orientation::{Horizontal, Vertical};
use log::warn;
use mpris_player::{Metadata, PlaybackStatus};
use util::format;
use crate::collection::model::Collection;
use crate::collection::song::Song;
use crate::collection::song::WithPath;
use crate::common::{EllipsizedLabelBuilder, gtk_box, util};
use crate::common::util::{or_none, PathString};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::controls::mpris::mpris_player;
use crate::controls::playbin::{go_delta_song, PLAYBIN, Playbin, URI};
use crate::controls::volume::volume_button;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

mod volume;
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
    let now_playing = gtk::Box::builder().orientation(Horizontal).build();
    let artist_album = gtk::Box::builder().orientation(Vertical).build();
    let unknown_album_file = IconTheme::for_display(&now_playing.display())
        .lookup_icon("audio-x-generic", &[], 128, 1, TextDirection::None, IconLookupFlags::empty()).file().unwrap();
    let album_picture = Picture::builder().content_fit(Contain).file(&unknown_album_file).build();
    now_playing.append(&album_picture);
    now_playing.append(&artist_album);
    let artist_label = Label::builder().ellipsized().build();
    artist_album.append(&artist_label);
    let song_label = Label::builder().ellipsized().build();
    let attr_list = AttrList::new();
    attr_list.insert(AttrInt::new_weight(Weight::Bold));
    song_label.set_attributes(Some(&attr_list));
    artist_album.append(&song_label);
    let play_pause = Button::builder().hexpand(true).build();
    play_pause.play();
    let position_label = Label::new(Some(&format(0)));
    let scale = Scale::builder().hexpand(true).build();
    scale.set_range(0.0, 1.0);
    let duration_label = Label::new(Some(&format(0)));
    let controls = gtk_box(Vertical);
    let position_box = gtk_box(Horizontal);
    let control_grid = Grid::builder().build();
    position_box.append(&position_label);
    position_box.append(&scale);
    position_box.append(&duration_label);
    let skip_backward = Button::builder().icon_name("media-skip-backward").tooltip_text("Previous")
        .hexpand(true).build();
    skip_backward.connect_clicked(|_| { go_delta_song(-1, true); });
    control_grid.attach(&skip_backward, 0, 0, 2, 1);
    let seek_backward = Button::builder().icon_name("media-seek-backward").tooltip_text("Seek 10s backward")
        .hexpand(true).build();
    seek_backward.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |_| { PLAYBIN.simple_seek(Duration::from_secs(10), false, &position_label, &scale); }
    });
    control_grid.attach(&seek_backward, 2, 0, 1, 1);
    control_grid.attach(&play_pause, 3, 0, 3, 1);
    let seek_forward = Button::builder().icon_name("media-seek-forward").tooltip_text("Seek 30s forward")
        .hexpand(true).build();
    seek_forward.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |_| { PLAYBIN.simple_seek(Duration::from_secs(30), true, &position_label, &scale); }
    });
    control_grid.attach(&seek_forward, 6, 0, 1, 1);
    let skip_forward = Button::builder().icon_name("media-skip-forward").tooltip_text("Next").hexpand(true).build();
    skip_forward.connect_clicked(|_| { go_delta_song(1, true); });
    control_grid.attach(&skip_forward, 7, 0, 2, 1);
    if !control_grid.display().default_seat().unwrap().capabilities().contains(SeatCapabilities::TOUCH) {
        control_grid.attach(&volume_button(|volume| { PLAYBIN.set_property("volume", volume); }), 9, 0, 1, 1);
    }
    controls.append(&now_playing);
    controls.append(&position_box);
    controls.append(&control_grid);
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
        let scale = scale.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error) = PLAYBIN.seek_internal(value as u64, &position_label, &scale) {
                    warn!("error trying to seek to {} {}", value, error);
                }
            }
            Inhibit(true)
        }
    });
    let wrapper = Wrapper::new(&controls);
    wrapper.connect_local(SONG_SELECTED, true, {
        let play_pause = play_pause.clone();
        move |params| {
            if let [_, current_song_path, collection_path] = &params {
                let playing = PLAYBIN.current_state() == Playing;
                PLAYBIN.set_state(Null).unwrap();
                PLAYBIN.set_uri(&collection_path.get::<String>().unwrap().to_path()
                    .join(current_song_path.get::<String>().unwrap().to_path()));
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
        let scale = scale.clone();
        move |delta_micros| {
            PLAYBIN.simple_seek(Duration::from_micros(delta_micros.abs() as u64), delta_micros >= 0,
                &position_label, &scale);
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
                                let mpris_player = mpris_player.clone();
                                move || {
                                    if let Some(position) = PLAYBIN.get_position() {
                                        position_label.set_label(&format(position));
                                        scale.set_value(position as f64);
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
                        let cover = (&song, &collection).path().parent().unwrap().join("cover.jpg");
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
