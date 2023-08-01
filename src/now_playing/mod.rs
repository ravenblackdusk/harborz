use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;
use adw::prelude::*;
use adw::WindowTitle;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{Continue, ElementExt, ElementExtManual, ObjectExt, GstObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, Image, Inhibit, Label, ProgressBar, Scale, ScrolledWindow, ScrollType};
use log::warn;
use mpris_player::{Metadata, PlaybackStatus};
use util::format;
use crate::body::Body;
use crate::body::collection::model::Collection;
use crate::song::{get_current_song, join_path, Song, WithCover};
use crate::song::WithPath;
use crate::common::{AdjustableScrolledWindow, EllipsizedLabelBuilder, util};
use crate::common::constant::BACK_ICON;
use crate::common::util::or_none;
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::now_playing::mpris::mpris_player;
use crate::now_playing::bottom_widget::Playable;
use crate::now_playing::playbin::{PLAYBIN, Playbin, URI};
use crate::db::get_connection;
use crate::now_playing::now_playing::NowPlaying;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

pub mod playbin;
mod mpris;
mod now_playing;
mod bottom_widget;
mod body;

pub fn create(song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, window_title: &WindowTitle,
    scrolled_window: &ScrolledWindow, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Button,
    header_body: &gtk::Box, body: &gtk::Box) -> Wrapper {
    let now_playing = Rc::new(RefCell::new(NowPlaying {
        cover: None,
        bottom_image: Image::builder().pixel_size(56).build(),
        body_image: Image::builder().pixel_size(360).build(),
        position: 0,
        duration: None,
        progress_bar: ProgressBar::builder().name("accent-progress").build(),
        scale: Scale::builder().hexpand(true).build(),
        bottom_position: Label::builder().label(&format(0)).margin_ellipsized(4).build(),
        body_position: Label::builder().label(&format(0)).build(),
    }));
    let (now_playing_body, duration_label) = body::create(now_playing.clone());
    let (bottom_widget, skip_song_gesture, image_click, song_label, artist_label, play_pause)
        = bottom_widget::create(now_playing.clone(), song_selected_body.clone(), window_title, scrolled_window,
        history.clone(), back_button);
    skip_song_gesture.connect_swipe(|_, velocity_x, velocity_y| {
        if velocity_x.abs() > velocity_y.abs() {
            PLAYBIN.go_delta_song(if velocity_x > 0.0 { -1 } else { 1 }, true);
        }
    });
    image_click.connect_released({
        let now_playing = now_playing.clone();
        let duration_label = duration_label.clone();
        let back_button = back_button.clone();
        let header_body = header_body.clone();
        let now_playing_body = now_playing_body.clone();
        move |_, _, _, _| {
            now_playing.borrow().update_other(&duration_label, &back_button, "go-down", &header_body,
                &now_playing_body);
        }
    });
    back_button.connect_clicked({
        let history = history.clone();
        let now_playing = now_playing.clone();
        let duration_label = duration_label.clone();
        let header_body = header_body.clone();
        let body = body.clone();
        let window_title = window_title.clone();
        let scrolled_window = scrolled_window.clone();
        move |back_button| {
            let mut history = history.borrow_mut();
            if back_button.icon_name().unwrap() == BACK_ICON {
                history.pop();
            } else {
                now_playing.borrow().update_other(&duration_label, &back_button, BACK_ICON, &header_body, &body);
            }
            back_button.set_visible(history.len() > 1);
            if let Some((body, adjust_scroll)) = history.last() {
                let Body { title, subtitle, widget, scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
                window_title.set_title(title.as_str());
                window_title.set_subtitle(subtitle.as_str());
                scrolled_window.set_child(Some((**widget).as_ref()));
                if *adjust_scroll { scrolled_window.adjust(&body_scroll_adjustment); }
            }
        }
    });
    play_pause.connect_clicked(|play_pause| {
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
    now_playing.borrow().scale.connect_change_value({
        let now_playing = now_playing.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error) = PLAYBIN.seek_internal(value as u64, now_playing.clone()) {
                    warn!("error trying to seek to {} {}", value, error);
                }
            }
            Inhibit(true)
        }
    });
    let mpris_player = mpris_player();
    mpris_player.connect_play_pause({
        let play_pause = play_pause.clone();
        move || { play_pause.emit_clicked(); }
    });
    mpris_player.connect_play({
        let play_pause = play_pause.clone();
        move || { if PLAYBIN.current_state() != Playing { play_pause.emit_clicked(); } }
    });
    mpris_player.connect_pause({
        let play_pause = play_pause.clone();
        move || { if PLAYBIN.current_state() != Paused { play_pause.emit_clicked(); } }
    });
    mpris_player.connect_seek({
        let now_playing = now_playing.clone();
        move |delta_micros| {
            PLAYBIN.simple_seek(Duration::from_micros(delta_micros.abs() as u64), delta_micros >= 0,
                now_playing.clone());
        }
    });
    let tracking_position = Rc::new(Cell::new(false));
    let once = Once::new();
    let wrapper = Wrapper::new(&bottom_widget);
    wrapper.connect_local(SONG_SELECTED, true, move |params| {
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
    });
    PLAYBIN.bus().unwrap().add_watch_local({
        let now_playing = now_playing.clone();
        let wrapper = wrapper.clone();
        move |_, message| {
            if message.src().map(|it| { it.name().starts_with("playbin3") }).unwrap_or(false) {
                match message.view() {
                    StateChanged(state_changed) => {
                        match state_changed.current() {
                            Playing => {
                                mpris_player.set_playback_status(PlaybackStatus::Playing);
                                if !tracking_position.get() {
                                    timeout_add_local(Duration::from_millis(500), {
                                        let now_playing = now_playing.clone();
                                        let tracking_position = tracking_position.clone();
                                        move || {
                                            if let Some(position) = PLAYBIN.get_position() {
                                                now_playing.borrow_mut().set_position(position);
                                            }
                                            tracking_position.set(PLAYBIN.current_state() == Playing
                                                || PLAYBIN.pending_state() == Playing);
                                            Continue(tracking_position.get())
                                        }
                                    });
                                    tracking_position.set(true);
                                }
                            }
                            Paused => { mpris_player.set_playback_status(PlaybackStatus::Paused); }
                            _ => {}
                        }
                    }
                    AsyncDone(_) => {
                        once.call_once(|| {
                            if let Ok((song, Config { current_song_position, .. }, _))
                                = get_current_song(&mut get_connection()) {
                                now_playing.borrow_mut().duration = Some(song.duration as u64);
                                PLAYBIN.seek_internal(current_song_position as u64, now_playing.clone()).unwrap();
                            }
                        });
                        now_playing.borrow_mut().set_duration(&duration_label);
                    }
                    DurationChanged(_) => { now_playing.borrow_mut().set_duration(&duration_label); }
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
                            let art_url = now_playing.borrow_mut().set_album_image(cover);
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
            }
            Continue(true)
        }
    }).unwrap();
    wrapper
}
