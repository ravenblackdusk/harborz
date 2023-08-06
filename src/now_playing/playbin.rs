use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use diesel::{Connection, QueryDsl, RunQueryDsl};
use gstreamer::{ClockTime, ElementFactory, Pipeline, SeekFlags};
use gstreamer::glib::{Cast, ObjectExt};
use gstreamer::prelude::{ElementExt, ElementExtManual};
use gstreamer::State::*;
use log::warn;
use once_cell::sync::Lazy;
use crate::body::collection::model::Collection;
use crate::config::Config;
use crate::db::get_connection;
use crate::now_playing::now_playing::NowPlaying;
use crate::schema::collections::dsl::collections;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, Song};
use crate::song::WithPath;

pub(in crate::now_playing) const URI: &'static str = "uri";
pub static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    let playbin = ElementFactory::make("playbin3").build().unwrap().downcast::<Pipeline>().unwrap();
    if let Ok((song, collection, _)) = songs.inner_join(collections).inner_join(config)
        .get_result::<(Song, Collection, Config)>(&mut get_connection()) {
        playbin.set_uri(&(&song, &collection).path());
        if let Err(error) = playbin.set_state(Paused) {
            warn!("error setting playbin state to Paused {}", error);
        }
    }
    playbin.connect("about-to-finish", true, {
        let playbin = playbin.clone();
        move |_| {
            playbin.go_delta_song(1, false);
            None
        }
    });
    playbin
});

pub trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn get_position(&self) -> Option<u64>;
    fn seek_internal(&self, value: u64, now_playing: Rc<RefCell<NowPlaying>>) -> anyhow::Result<()>;
    fn simple_seek(&self, delta: Duration, forward: bool, now_playing: Rc<RefCell<NowPlaying>>);
    fn go_delta_song(&self, delta: i32, now: bool);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property(URI, format!("file:{}", uri.to_str().unwrap()));
    }
    fn get_position(&self) -> Option<u64> {
        PLAYBIN.query_position().map(ClockTime::nseconds)
    }
    fn seek_internal(&self, value: u64, now_playing: Rc<RefCell<NowPlaying>>) -> anyhow::Result<()> {
        self.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value))?;
        Ok(now_playing.borrow_mut().set_position(value))
    }
    fn simple_seek(&self, delta: Duration, forward: bool, now_playing: Rc<RefCell<NowPlaying>>) {
        if let Some(position) = self.get_position() {
            let nanos = delta.as_nanos() as i64;
            self.seek_internal(
                ((position as i64) + if forward { nanos } else { -nanos })
                    .clamp(0, now_playing.clone().borrow().duration as i64) as u64, now_playing,
            ).unwrap();
        }
    }
    fn go_delta_song(&self, delta: i32, now: bool) {
        get_connection().transaction(|connection| {
            if let Ok((Some(current_song_id_int), artist_string, album_string)) = config.inner_join(songs)
                .select((current_song_id, artist, album))
                .get_result::<(Option<i32>, Option<String>, Option<String>)>(connection) {
                let song_collections = get_current_album(artist_string.map(Rc::new), album_string.map(Rc::new), connection);
                let delta_song_index = song_collections.iter().position(|(song, _)| { song.id == current_song_id_int })
                    .unwrap() as i32 + delta;
                if delta_song_index >= 0 && delta_song_index < song_collections.len() as i32 {
                    let (delta_song, delta_collection) = &song_collections[delta_song_index as usize];
                    let playing = self.current_state() == Playing;
                    if now { self.set_state(Null).unwrap(); }
                    self.set_uri(&(delta_song, delta_collection).path());
                    if now { self.set_state(if playing { Playing } else { Paused }).unwrap(); }
                }
            }
            anyhow::Ok(())
        }).unwrap();
    }
}
