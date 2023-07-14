use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};
use diesel::{BoolExpressionMethods, ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, SqliteConnection};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use gstreamer::ClockTime;
use gstreamer::tags::{Album, AlbumArtist, Artist, DateTime, Genre, Title, TrackNumber};
use gstreamer_pbutils::Discoverer;
use gtk::glib::{Continue, timeout_add};
use once_cell::sync::Lazy;
use walkdir::WalkDir;
use crate::collection::model::Collection;
use crate::common::util::PathString;
use crate::schema::collections::table as collections;
use crate::schema::songs::*;
use crate::schema::songs::dsl::songs;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::songs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Song {
    pub id: i32,
    pub path: String,
    pub collection_id: i32,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub datetime: Option<i64>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub album_artist: Option<String>,
    pub duration: i64,
}

impl Song {
    pub fn title_str(&self) -> &str {
        self.title.as_deref().unwrap_or(self.path.to_path().file_name().unwrap().to_str().unwrap())
    }
}

pub trait WithPath {
    fn path(&self) -> PathBuf;
}

impl WithPath for (&Song, &Collection) {
    fn path(&self) -> PathBuf {
        let (song, collection) = self;
        collection.path.to_path().join(song.path.to_path())
    }
}

static DISCOVERER: Lazy<Discoverer> = Lazy::new(|| { Discoverer::new(ClockTime::from_seconds(30)).unwrap() });

pub(in crate::collection) enum ImportProgress {
    CollectionStart(i32),
    Fraction(f64),
    CollectionEnd(i32, String),
}

pub(in crate::collection) fn import_songs(collection: &Collection, sender: Sender<ImportProgress>,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>) -> Option<SystemTime> {
    sender.send(ImportProgress::CollectionStart(collection.id)).unwrap();
    let total = WalkDir::new(&collection.path).into_iter().count() as f64;
    let count = Arc::new(Mutex::new(0.0));
    timeout_add(Duration::from_millis(200), {
        let sender = sender.clone();
        let count = count.clone();
        move || {
            let count = count.lock().unwrap();
            sender.send(ImportProgress::Fraction(*count / total)).unwrap();
            Continue(*count < total)
        }
    });
    let result = WalkDir::new(&collection.path).into_iter().filter_map(|entry_result| {
        *count.lock().unwrap() += 1.0;
        let entry = entry_result.unwrap();
        entry.file_type().is_file().then_some(entry)
    }).map(|entry| -> anyhow::Result<_> {
        Ok(if let Ok(discoverer_info) = DISCOVERER.discover_uri(format!("file:{}", entry.path().to_str().unwrap()).as_str()) {
            if discoverer_info.video_streams().is_empty() && !discoverer_info.audio_streams().is_empty() {
                let tag_list = discoverer_info.tags().unwrap();
                let tag_list_ref = tag_list.as_ref();
                if let Err(Error::NotFound) = insert_or_ignore_into(songs).values((
                    path.eq(entry.path().strip_prefix(&collection.path)?.to_str().unwrap()),
                    title.eq(tag_list_ref.get::<Title>().map(|it| { it.get().to_owned() })),
                    artist.eq(tag_list_ref.get::<Artist>().map(|it| { it.get().to_owned() })),
                    album.eq(tag_list_ref.get::<Album>().map(|it| { it.get().to_owned() })),
                    datetime.eq(tag_list_ref.get::<DateTime>().and_then(|date_time| { date_time.get().microsecond() })
                        .map(|date_time| { Duration::from_micros(date_time as u64).as_nanos() as i64 })),
                    genre.eq(tag_list_ref.get::<Genre>().map(|it| { it.get().to_owned() })),
                    track_number.eq(tag_list_ref.get::<TrackNumber>().map(|it| { it.get() as i32 })),
                    album_artist.eq(tag_list_ref.get::<AlbumArtist>().map(|it| { it.get().to_owned() })),
                    duration.eq(discoverer_info.duration().unwrap().nseconds() as i64),
                    collection_id.eq(collection.id),
                )).execute(connection) {
                    None
                } else {
                    Some(entry.metadata()?.modified()?)
                }
            } else {
                None
            }
        } else {
            None
        })
    }).filter_map(|result| { result.unwrap() }).max();
    sender.send(ImportProgress::CollectionEnd(collection.id, collection.path.to_owned())).unwrap();
    result
}

pub fn get_current_album(artist_string: &Option<String>, album_string: &Option<String>,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>) -> Vec<(Song, Collection)> {
    songs.inner_join(collections).filter(artist.eq(artist_string).and(album.eq(album_string)))
        .order_by((track_number, id)).get_results::<(Song, Collection)>(connection).unwrap()
}
