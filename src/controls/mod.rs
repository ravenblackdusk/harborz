use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;
use adw::prelude::*;
use adw::WindowTitle;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::ClockTime;
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{Continue, ElementExt, ElementExtManual, ObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, CssProvider, GestureLongPress, Image, Inhibit, Label, ProgressBar, Scale, ScrolledWindow, ScrollType, style_context_add_provider_for_display, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::Orientation::Vertical;
use log::warn;
use mpris_player::{Metadata, PlaybackStatus};
use util::format;
use crate::body::Body;
use crate::body::collection::model::Collection;
use crate::song::{get_current_song, join_path, Song, WithCover};
use crate::song::WithPath;
use crate::common::{BoldLabelBuilder, EllipsizedLabelBuilder, util};
use crate::common::constant::UNKNOWN_ALBUM;
use crate::common::util::or_none;
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::controls::mpris::mpris_player;
use crate::controls::playbin::{go_delta_song, PLAYBIN, Playbin, URI};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

pub mod playbin;
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

fn update_duration(duration: &mut Option<u64>, label: &Label, scale: &Scale) {
    *duration = PLAYBIN.query_duration().map(ClockTime::nseconds);
    if let Some(duration) = duration {
        label.set_label(&format(*duration));
        scale.set_range(0.0, *duration as f64);
    }
}

pub fn media_controls(song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, window_title: &WindowTitle,
    scrolled_window: &ScrolledWindow, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Option<Button>)
    -> Wrapper {
    let once = Once::new();
    let mpris_player = mpris_player();
    let now_playing_and_progress = gtk::Box::builder().orientation(Vertical).name("accent-bg").build();
    let now_playing = gtk::Box::builder().margin_start(8).margin_end(8).margin_top(8).margin_bottom(8).build();
    let css_provider = CssProvider::new();
    css_provider.load_from_data("#accent-bg { background-color: @accent_bg_color; } \
    #accent-progress progress { background-color: @accent_fg_color; }");
    style_context_add_provider_for_display(&now_playing_and_progress.display(), &css_provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION);
    let progress_bar = ProgressBar::builder().name("accent-progress").build();
    progress_bar.add_css_class("osd");
    let song_info = gtk::Box::builder().orientation(Vertical).margin_start(4).build();
    let album_image = Image::builder().pixel_size(56).build();
    let long_press = GestureLongPress::new();
    long_press.connect_pressed({
        let song_selected_body = song_selected_body.clone();
        let window_title = window_title.clone();
        let scrolled_window = scrolled_window.clone();
        let history = history.clone();
        let back_button = back_button.clone();
        move |_, _, _| {
            if let Some(body) = song_selected_body.borrow().as_ref() {
                body.clone().set(&window_title, &scrolled_window, history.clone(), &back_button);
            }
        }
    });
    album_image.add_controller(long_press);
    now_playing.append(&album_image);
    now_playing.append(&song_info);
    let play_pause = Button::builder().width_request(40).build();
    play_pause.play();
    let toolbar = gtk::Box::builder().build();
    toolbar.add_css_class("toolbar");
    let skip_backward = Button::builder().icon_name("media-skip-backward").tooltip_text("Previous").hexpand(true)
        .build();
    skip_backward.connect_clicked(|_| { go_delta_song(-1, true); });
    let skip_forward = Button::builder().icon_name("media-skip-forward").tooltip_text("Next").hexpand(true).build();
    skip_forward.connect_clicked(|_| { go_delta_song(1, true); });
    toolbar.append(&skip_backward);
    toolbar.append(&play_pause);
    toolbar.append(&skip_forward);
    now_playing.append(&toolbar);
    let song_label = Label::builder().margin_ellipsized(4).bold().build();
    let artist_label = Label::builder().margin_ellipsized(4).build();
    song_info.append(&song_label);
    song_info.append(&artist_label);
    let position_label = Label::builder().label(&format(0)).margin_ellipsized(4).build();
    let duration_label = Label::new(Some(&format(0)));
    song_info.append(&position_label);
    let scale = Scale::builder().hexpand(true).build();
    scale.set_range(0.0, 1.0);
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
    let mut duration: Option<u64> = None;
    scale.connect_change_value({
        let position_label = position_label.clone();
        let progress_bar = progress_bar.clone();
        let scale = scale.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error)
                    = PLAYBIN.seek_internal(value as u64, &position_label, &progress_bar, duration, &scale) {
                    warn!("error trying to seek to {} {}", value, error);
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
                *song_selected_body.borrow_mut() = history.borrow().last().map(|(body, _)| { body }).cloned();
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
            PLAYBIN.simple_seek(Duration::from_micros(delta_micros.abs() as u64), duration, delta_micros >= 0,
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
                            timeout_add_local(Duration::from_millis(500), {
                                let position_label = position_label.clone();
                                let scale = scale.clone();
                                let progress_bar = progress_bar.clone();
                                move || {
                                    if let Some(position) = PLAYBIN.get_position() {
                                        position_label.set_label(&format(position));
                                        scale.set_value(position as f64);
                                        if let Some(duration) = duration {
                                            progress_bar.set_fraction(position as f64 / duration as f64);
                                        }
                                    }
                                    Continue(PLAYBIN.current_state() == Playing || PLAYBIN.pending_state() == Playing)
                                }
                            });
                        }
                        Paused => { mpris_player.set_playback_status(PlaybackStatus::Paused); }
                        _ => {}
                    }
                }
                AsyncDone(_) => {
                    once.call_once(|| {
                        if let Ok((song, Config { current_song_position, .. }, _))
                            = get_current_song(&mut get_connection()) {
                            PLAYBIN.seek_internal(current_song_position as u64, &position_label, &progress_bar,
                                Some(song.duration as u64), &scale).unwrap();
                        }
                    });
                    update_duration(&mut duration, &duration_label, &scale);
                }
                DurationChanged(_) => { update_duration(&mut duration, &duration_label, &scale); }
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
                            album_image.set_from_file(Some(&cover));
                            cover.to_str().map(|it| { format!("file:{}", it) })
                        } else {
                            album_image.set_icon_name(Some(UNKNOWN_ALBUM));
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
