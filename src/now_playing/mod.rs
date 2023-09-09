use std::cell::{Cell, RefCell};
use std::mem::forget;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;
use adw::glib::Propagation;
use adw::prelude::*;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::glib::{ControlFlow::*, timeout_add_local};
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{ElementExt, ElementExtManual, GstObjectExt, ObjectExt as GstreamerObject};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{EventSequenceState, ScrollType};
use log::warn;
use mpris_player::{Metadata, PlaybackStatus};
use crate::body::artists::artists;
use crate::body::Body;
use crate::body::collection::model::Collection;
use crate::common::AdjustableScrolledWindow;
use crate::common::constant::BACK_ICON;
use crate::common::gesture::{Direction, DirectionSwipe};
use crate::common::state::State;
use crate::common::util::or_none;
use crate::config::{Config, update_now_playing_body_realized};
use crate::db::get_connection;
use crate::now_playing::mpris::mpris_player;
use crate::now_playing::now_playing::{NowPlaying, Playable};
use crate::now_playing::playbin::{PLAYBIN, Playbin, URI};
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;
use crate::song::{get_current_song, Song, WithImage};
use crate::song::WithPath;

pub mod playbin;
mod mpris;
mod now_playing;
mod bottom_widget;
mod body;

fn go_delta_song(velocity_x: f64) {
    PLAYBIN.go_delta_song(if velocity_x > 0.0 { -1 } else { 1 }, true);
}

pub fn create(song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, state: Rc<State>, body: &gtk::Box)
    -> (gtk::Box, gtk::Box, Rc<RefCell<NowPlaying>>) {
    let now_playing = Rc::new(RefCell::new(NowPlaying::new()));
    let (now_playing_body, body_swipe_gesture) = body::create(now_playing.clone());
    let (bottom_widget, bottom_swipe_gesture, image_click)
        = bottom_widget::create(now_playing.clone(), song_selected_body.clone(), state.clone());
    let realize_bottom = {
        let now_playing = now_playing.clone();
        let state = state.clone();
        let body = body.clone();
        move || {
            update_now_playing_body_realized(false);
            now_playing.borrow().update_other(state.clone(), BACK_ICON, &body);
        }
    };
    let show_last_history = {
        let state = state.clone();
        move || {
            if let Some((body, adjust_scroll)) = state.history.borrow().last() {
                body.clone().set(state.clone());
                let Body { scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
                if *adjust_scroll { state.scrolled_window.adjust(&body_scroll_adjustment); }
            }
        }
    };
    body_swipe_gesture.connect_direction_swipe({
        let realize_bottom = realize_bottom.clone();
        let show_last_history = show_last_history.clone();
        move |gesture, velocity_x, velocity_y, direction_swipe| {
            if direction_swipe(Direction::Horizontal) {
                gesture.set_state(EventSequenceState::Claimed);
                go_delta_song(velocity_x);
            } else if velocity_y > 0.0 && direction_swipe(Direction::Vertical) {
                gesture.set_state(EventSequenceState::Claimed);
                realize_bottom();
                show_last_history();
            }
        }
    });
    let realize_body = {
        let now_playing = now_playing.clone();
        let state = state.clone();
        let now_playing_body = now_playing_body.clone();
        move || {
            now_playing.borrow().realize_body(state.clone(), &now_playing_body);
            update_now_playing_body_realized(true);
        }
    };
    bottom_swipe_gesture.connect_direction_swipe({
        let realize_body = realize_body.clone();
        move |gesture, velocity_x, velocity_y, direction_swipe| {
            if direction_swipe(Direction::Horizontal) {
                gesture.set_state(EventSequenceState::Claimed);
                go_delta_song(velocity_x);
            } else if velocity_y < 0.0 && direction_swipe(Direction::Vertical) {
                gesture.set_state(EventSequenceState::Claimed);
                realize_body();
            }
        }
    });
    image_click.connect_released(move |gesture, _, _, _| {
        gesture.set_state(EventSequenceState::Claimed);
        realize_body();
    });
    state.back_button.connect_clicked({
        let state = state.clone();
        move |back_button| {
            if state.history.borrow().is_empty() {
                Rc::new(artists(state.clone())).set_with_history(state.clone());
            } else {
                if back_button.icon_name().unwrap() == BACK_ICON {
                    state.history.borrow_mut().pop();
                } else {
                    realize_bottom();
                }
                show_last_history();
            }
        }
    });
    for play_pause in vec![now_playing.borrow().bottom_play_pause.clone(),
        now_playing.borrow().body_play_pause.clone()] {
        play_pause.connect_clicked(|play_pause| {
            if PLAYBIN.current_state() == Playing {
                PLAYBIN.set_state(Paused).unwrap();
                play_pause.play();
            } else {
                match PLAYBIN.set_state(Playing) {
                    Ok(_) => { play_pause.pause(); }
                    Err(error) => {
                        warn!("error trying to play [{:?}] [{error}]", PLAYBIN.property::<Option<String>>(URI));
                    }
                }
            }
        });
    }
    now_playing.borrow().scale.connect_change_value({
        let now_playing = now_playing.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error) = PLAYBIN.seek_internal(value as u64, now_playing.clone()) {
                    warn!("error trying to seek to [{value}] [{error}]");
                }
            }
            Propagation::Stop
        }
    });
    let mpris_player = mpris_player();
    mpris_player.connect_play_pause({
        let now_playing = now_playing.clone();
        move || { now_playing.borrow().click_play_pause(); }
    });
    mpris_player.connect_play({
        let now_playing = now_playing.clone();
        move || { if PLAYBIN.current_state() != Playing { now_playing.borrow().click_play_pause(); } }
    });
    mpris_player.connect_pause({
        let now_playing = now_playing.clone();
        move || { if PLAYBIN.current_state() != Paused { now_playing.borrow().click_play_pause(); } }
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
    state.window_actions.song_selected.action.connect_activate({
        let now_playing = now_playing.clone();
        let state = state.clone();
        move |_, params| {
            let playing = PLAYBIN.current_state() == Playing;
            PLAYBIN.set_state(Null).unwrap();
            PLAYBIN.set_uri_str(params.unwrap().str().unwrap());
            if playing {
                PLAYBIN.set_state(Playing).unwrap();
            } else {
                now_playing.borrow().click_play_pause();
            }
            *song_selected_body.borrow_mut() = state.history.borrow().last().map(|(body, _)| { body }).cloned();
        }
    });
    forget(PLAYBIN.bus().unwrap().add_watch_local({
        let now_playing = now_playing.clone();
        let state = state.clone();
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
                                            if tracking_position.get() { Continue } else { Break }
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
                                now_playing.borrow_mut().duration = song.duration as u64;
                                PLAYBIN.seek_internal(current_song_position as u64, now_playing.clone()).unwrap();
                            }
                        });
                        now_playing.borrow_mut().set_duration();
                    }
                    DurationChanged(_) => { now_playing.borrow_mut().set_duration(); }
                    StreamStart(_) => {
                        let uri = &PLAYBIN.property::<String>("current-uri")[5.. /* remove "file:" */];
                        get_connection().transaction(|connection| {
                            let (collection, song) = collections.inner_join(songs)
                                .filter(path.concat("/").concat(song_path).eq(uri))
                                .get_result::<(Collection, Song)>(connection)?;
                            update(config).set(current_song_id.eq(song.id)).execute(connection)?;
                            let title = song.title_str().to_owned();
                            now_playing.borrow_mut().set_song_info(state.clone(), &title, or_none(&song.artist));
                            let cover = (&song, &collection).path().cover();
                            let art_url = now_playing.borrow_mut().set_album_image(cover);
                            state.window_actions.stream_started.activate(song.id);
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
            Continue
        }
    }).unwrap());
    (now_playing_body, bottom_widget, now_playing)
}
