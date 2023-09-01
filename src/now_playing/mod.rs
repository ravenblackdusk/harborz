use std::cell::{Cell, RefCell};
use std::mem::forget;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;
use adw::gio::SimpleAction;
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
use crate::body::Body;
use crate::body::collection::model::Collection;
use crate::common::action::SONG_SELECTED;
use crate::common::AdjustableScrolledWindow;
use crate::common::constant::BACK_ICON;
use crate::common::state::State;
use crate::common::util::or_none;
use crate::common::wrapper::{STREAM_STARTED, Wrapper};
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
use crate::song::{get_current_song, Song, WithCover};
use crate::song::WithPath;

pub mod playbin;
mod mpris;
mod now_playing;
mod bottom_widget;
mod body;

pub fn create(song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, state: Rc<State>, body: &gtk::Box)
    -> (gtk::Box, Wrapper, Rc<RefCell<NowPlaying>>) {
    let now_playing = Rc::new(RefCell::new(NowPlaying::new()));
    let (now_playing_body, body_skip_song_gesture) = body::create(now_playing.clone());
    let (bottom_widget, bottom_skip_song_gesture, image_click)
        = bottom_widget::create(now_playing.clone(), song_selected_body.clone(), state.clone());
    for skip_song_gesture in vec![body_skip_song_gesture, bottom_skip_song_gesture] {
        skip_song_gesture.connect_swipe(|gesture, velocity_x, velocity_y| {
            if velocity_x.abs() * 0.1 > velocity_y.abs() {
                gesture.set_state(EventSequenceState::Claimed);
                PLAYBIN.go_delta_song(if velocity_x > 0.0 { -1 } else { 1 }, true);
            }
        });
    }
    image_click.connect_released({
        let now_playing = now_playing.clone();
        let state = state.clone();
        let now_playing_body = now_playing_body.clone();
        move |gesture, _, _, _| {
            gesture.set_state(EventSequenceState::Claimed);
            now_playing.borrow().realize_body(state.clone(), &now_playing_body);
            update_now_playing_body_realized(true);
        }
    });
    let wrapper = Wrapper::new(&bottom_widget);
    state.back_button.connect_clicked({
        let now_playing = now_playing.clone();
        let state = state.clone();
        let body = body.clone();
        let wrapper = wrapper.clone();
        move |back_button| {
            if state.history.borrow().is_empty() {
                Rc::new(Body::artists(state.clone(), &wrapper)).set_with_history(state.clone());
            } else {
                let mut history = state.history.borrow_mut();
                if back_button.icon_name().unwrap() == BACK_ICON {
                    history.pop();
                } else {
                    update_now_playing_body_realized(false);
                    now_playing.borrow().update_other(state.clone(), BACK_ICON, &body);
                }
                if let Some((body, adjust_scroll)) = history.last() {
                    body.clone().set(state.clone());
                    let Body { scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
                    if *adjust_scroll { state.scrolled_window.adjust(&body_scroll_adjustment); }
                }
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
                    Err(error) => { warn!("error trying to play [{}] [{}]", PLAYBIN.property::<String>(URI), error); }
                }
            }
        });
    }
    now_playing.borrow().scale.connect_change_value({
        let now_playing = now_playing.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error) = PLAYBIN.seek_internal(value as u64, now_playing.clone()) {
                    warn!("error trying to seek to [{}] [{}]", value, error);
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
    let song_selected = SimpleAction::new(SONG_SELECTED, Some(&String::static_variant_type()));
    state.window.add_action(&song_selected);
    song_selected.connect_activate({
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
                            now_playing.borrow_mut().set_song_info(&title, or_none(&song.artist), &state.window_title);
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
            Continue
        }
    }).unwrap());
    (now_playing_body, wrapper, now_playing)
}
