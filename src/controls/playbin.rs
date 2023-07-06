use std::path::PathBuf;
use std::time::Duration;
use diesel::{Connection, QueryDsl, RunQueryDsl};
use gstreamer::{ClockTime, ElementFactory, Pipeline, SeekFlags};
use gstreamer::glib::{Cast, ObjectExt};
use gstreamer::prelude::{ElementExt, ElementExtManual};
use gstreamer::State::{Null, Playing};
use gtk::{Label, Scale};
use gtk::prelude::RangeExt;
use once_cell::sync::Lazy;
use crate::collection::song::get_current_album;
use crate::common::util::{format, PathString};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

pub(in crate::controls) const URI: &'static str = "uri";

pub(in crate::controls) static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    let playbin = ElementFactory::make("playbin3").build().unwrap().downcast::<Pipeline>().unwrap();
    let path_buf = songs.inner_join(collections).inner_join(config).select((path, song_path))
        .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, current_song_path)| {
        collection_path.to_path().join(current_song_path.to_path())
    }).unwrap_or(PathBuf::from(""));
    playbin.set_uri(&path_buf);
    playbin.connect("about-to-finish", true, |_| {
        go_delta_song(1, false);
        None
    });
    playbin
});

pub(in crate::controls) trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn get_position(&self) -> Option<u64>;
    fn seek_internal(&self, value: u64, label: &Label, scale: &Scale) -> anyhow::Result<()>;
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, scale: &Scale);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property(URI, format!("file:{}", uri.to_str().unwrap()));
    }
    fn get_position(&self) -> Option<u64> {
        PLAYBIN.query_position().map(ClockTime::nseconds)
    }
    fn seek_internal(&self, value: u64, label: &Label, scale: &Scale) -> anyhow::Result<()> {
        self.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value))?;
        label.set_label(&format(value));
        Ok(scale.set_value(value as f64))
    }
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, scale: &Scale) {
        if let Some(position) = self.get_position() {
            let nanos = duration.as_nanos() as i64;
            self.seek_internal(
                ((position as i64) + if forward { nanos } else { -nanos })
                    .clamp(0, PLAYBIN.query_duration().map(ClockTime::nseconds).unwrap() as i64) as u64,
                &label, &scale,
            ).unwrap();
        }
    }
}

pub(in crate::controls) fn go_delta_song(delta: i32, now: bool) {
    get_connection().transaction(|connection| {
        if let Ok((Some(current_song_id_int), artist_string, album_string)) = config.inner_join(songs)
            .select((current_song_id, artist, album))
            .get_result::<(Option<i32>, Option<String>, Option<String>)>(connection) {
            let song_collections = get_current_album(&artist_string, &album_string, connection);
            let delta_song_index = song_collections.iter().position(|(song, _)| { song.id == current_song_id_int })
                .unwrap() as i32 + delta;
            if delta_song_index >= 0 && delta_song_index < song_collections.len() as i32 {
                let (delta_song, delta_collection) = &song_collections[delta_song_index as usize];
                let playing = PLAYBIN.current_state() == Playing;
                if now { PLAYBIN.set_state(Null).unwrap(); }
                PLAYBIN.set_uri(&delta_collection.path.to_path().join(delta_song.path.to_path()));
                if now && playing { PLAYBIN.set_state(Playing).unwrap(); }
            }
        }
        anyhow::Ok(())
    }).unwrap();
}
