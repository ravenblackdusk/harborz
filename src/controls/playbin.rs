use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use adw::prelude::RangeExt;
use diesel::{Connection, QueryDsl, RunQueryDsl};
use gstreamer::{ClockTime, ElementFactory, Pipeline, SeekFlags};
use gstreamer::glib::{Cast, ObjectExt};
use gstreamer::prelude::{ElementExt, ElementExtManual};
use gstreamer::State::*;
use gtk::{Label, ProgressBar, Scale};
use mpris_player::MprisPlayer;
use once_cell::sync::Lazy;
use crate::body::collection::model::Collection;
use crate::common::util::format;
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, Song};
use crate::song::WithPath;

pub(in crate::controls) const URI: &'static str = "uri";
pub static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    let playbin = ElementFactory::make("playbin3").build().unwrap().downcast::<Pipeline>().unwrap();
    if let Ok((song, collection, _)) = songs.inner_join(collections).inner_join(config)
        .get_result::<(Song, Collection, Config)>(&mut get_connection()) {
        playbin.set_uri(&(&song, &collection).path());
        playbin.set_state(Paused).unwrap();
    }
    playbin.connect("about-to-finish", true, |_| {
        go_delta_song(1, false);
        None
    });
    playbin
});

pub trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn get_position(&self) -> Option<u64>;
    fn get_duration(&self) -> Option<u64>;
    fn seek_internal(&self, value: u64, label: &Label, progress_bar: &ProgressBar, duration: u64, scale: &Scale)
        -> anyhow::Result<()>;
    fn seek_internal_and_mpris(&self, value: u64, label: &Label, progress_bar: &ProgressBar, duration: u64,
        scale: &Scale, mpris_player: &MprisPlayer) -> anyhow::Result<()>;
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, progress_bar: &ProgressBar, scale: &Scale);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property(URI, format!("file:{}", uri.to_str().unwrap()));
    }
    fn get_position(&self) -> Option<u64> {
        PLAYBIN.query_position().map(ClockTime::nseconds)
    }
    fn get_duration(&self) -> Option<u64> {
        PLAYBIN.query_duration().map(ClockTime::nseconds)
    }
    fn seek_internal(&self, value: u64, label: &Label, progress_bar: &ProgressBar, duration: u64, scale: &Scale)
        -> anyhow::Result<()> {
        self.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value))?;
        label.set_label(&format(value));
        progress_bar.set_fraction(value as f64 / duration as f64);
        Ok(scale.set_value(value as f64))
    }
    fn seek_internal_and_mpris(&self, value: u64, label: &Label, progress_bar: &ProgressBar, duration: u64,
        scale: &Scale, mpris_player: &MprisPlayer) -> anyhow::Result<()> {
        self.seek_internal(value, label, progress_bar, duration, scale)?;
        Ok(mpris_player.set_position(value as i64))
    }
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, progress_bar: &ProgressBar, scale: &Scale) {
        if let Some(position) = self.get_position() {
            let nanos = duration.as_nanos() as i64;
            let duration = self.get_duration().unwrap();
            self.seek_internal(
                ((position as i64) + if forward { nanos } else { -nanos }).clamp(0, duration as i64) as u64, label,
                progress_bar, duration, scale
            ).unwrap();
        }
    }
}

pub(in crate::controls) fn go_delta_song(delta: i32, now: bool) {
    get_connection().transaction(|connection| {
        if let Ok((Some(current_song_id_int), artist_string, album_string)) = config.inner_join(songs)
            .select((current_song_id, artist, album))
            .get_result::<(Option<i32>, Option<String>, Option<String>)>(connection) {
            let song_collections = get_current_album(artist_string.map(Rc::new), album_string.map(Rc::new), connection);
            let delta_song_index = song_collections.iter().position(|(song, _)| { song.id == current_song_id_int })
                .unwrap() as i32 + delta;
            if delta_song_index >= 0 && delta_song_index < song_collections.len() as i32 {
                let (delta_song, delta_collection) = &song_collections[delta_song_index as usize];
                let playing = PLAYBIN.current_state() == Playing;
                if now { PLAYBIN.set_state(Null).unwrap(); }
                PLAYBIN.set_uri(&(delta_song, delta_collection).path());
                if now && playing { PLAYBIN.set_state(Playing).unwrap(); }
            }
        }
        anyhow::Ok(())
    }).unwrap();
}
