use std::cmp::max;
use std::fmt::Debug;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use async_std::task;
use diesel::{ExpressionMethods, insert_into, QueryDsl, QueryResult, RunQueryDsl, SqliteConnection, update};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use gstreamer::ClockTime;
use gstreamer::tags::*;
use gstreamer_pbutils::Discoverer;
use log::info;
use once_cell::sync::Lazy;
use walkdir::{DirEntry, WalkDir};
use crate::body::collection::model::Collection;
use crate::common::util::PathString;
use crate::config::Config;
use crate::schema::collections::{modified, table as collections};
use crate::schema::config::dsl::config;
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
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub album_volume: Option<i32>,
    pub album_artist: Option<String>,
    pub duration: i64,
    pub lyrics: Option<String>,
}

impl Song {
    pub fn title_str(&self) -> &str {
        self.title.as_deref().unwrap_or(self.path.to_path().file_name().unwrap().to_str().unwrap())
    }
}

pub fn join_path(collection_path: &String, song_path: &String) -> PathBuf {
    collection_path.to_path().join(song_path.to_path())
}

pub trait WithPath {
    fn path(&self) -> PathBuf;
}

impl WithPath for (&Song, &Collection) {
    fn path(&self) -> PathBuf {
        let (song, collection) = self;
        join_path(&collection.path, &song.path)
    }
}

pub trait WithImage {
    fn cover(&self) -> PathBuf;
    fn logo(&self) -> PathBuf;
    fn sibling_logo(&self) -> PathBuf;
    fn photo(&self) -> PathBuf;
    fn sibling_photo(&self) -> PathBuf;
}

fn join_parent(path_ref: impl AsRef<Path>, file_name: &str) -> PathBuf {
    path_ref.as_ref().parent().unwrap().join(file_name)
}

fn join_grandparent(path_ref: impl AsRef<Path>, file_name: &str) -> PathBuf {
    join_parent(path_ref.as_ref().parent().unwrap(), file_name)
}

const LOGO: &'static str = "logo.jpg";
const PHOTO: &'static str = "photo.jpg";

impl<P: AsRef<Path>> WithImage for P {
    fn cover(&self) -> PathBuf {
        join_parent(self, "cover.jpg")
    }
    fn logo(&self) -> PathBuf {
        join_grandparent(self, LOGO)
    }
    fn sibling_logo(&self) -> PathBuf {
        join_parent(self, LOGO)
    }
    fn photo(&self) -> PathBuf {
        join_grandparent(self, PHOTO)
    }
    fn sibling_photo(&self) -> PathBuf {
        join_parent(self, PHOTO)
    }
}

static DISCOVERER: Lazy<Discoverer> = Lazy::new(|| { Discoverer::new(ClockTime::from_seconds(30)).unwrap() });

pub enum ImportProgress {
    CollectionStart,
    Fraction(f64),
    CollectionEnd(Arc<RwLock<Collection>>),
}

trait StrTag<'a> {
    fn as_str(&self) -> Option<&'a str>;
}

impl<'a> StrTag<'a> for Option<&'a TagValue<&str>> {
    fn as_str(&self) -> Option<&'a str> {
        match self {
            Some(tag_value) => { Some(tag_value.get()) }
            None => { None }
        }
    }
}

fn walk_newer_than(collection: &Arc<RwLock<Collection>>, last_modified: Option<SystemTime>)
    -> Box<dyn Iterator<Item=walkdir::Result<DirEntry>>> {
    let into_iter = WalkDir::new(&collection.read().unwrap().path).into_iter();
    if let Some(last_modified) = last_modified {
        Box::new(into_iter.filter_entry(move |entry| {
            let metadata = entry.metadata().unwrap();
            max(metadata.created().unwrap(), metadata.modified().unwrap()) > last_modified
        }))
    } else {
        Box::new(into_iter)
    }
}

pub fn import_songs(collection: Arc<RwLock<Collection>>, sender: Sender<ImportProgress>,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>) -> anyhow::Result<()> {
    sender.send(ImportProgress::CollectionStart)?;
    let last_modified = collection.read().unwrap().modified
        .map(|it| { UNIX_EPOCH.add(Duration::from_nanos(it as u64)) });
    let total = walk_newer_than(&collection, last_modified).count();
    info!("importing [{total}] new files to collection [{collection:?}]");
    let total_f64 = total as f64;
    let count = Arc::new(AtomicUsize::new(0));
    task::spawn({
        let count = count.clone();
        let sender = sender.clone();
        async move {
            loop {
                task::sleep(Duration::from_millis(500)).await;
                let count = count.load(Ordering::Relaxed);
                if count == total { break; }
                sender.send(ImportProgress::Fraction(count as f64 / total_f64)).unwrap();
            }
        }
    });
    let max_modified = walk_newer_than(&collection, last_modified).filter_map(|entry_result| {
        task::block_on({
            let count = count.clone();
            async move { count.fetch_add(1, Ordering::Relaxed); }
        });
        let entry = entry_result.unwrap();
        entry.file_type().is_file().then_some(entry)
    }).map(|entry| -> anyhow::Result<_> {
        Ok(if let Ok(discoverer_info) = DISCOVERER
            .discover_uri(format!("file:{}", entry.path().to_str().unwrap()).as_str()) {
            if discoverer_info.video_streams().is_empty() && !discoverer_info.audio_streams().is_empty() {
                let tag_list = discoverer_info.tags().unwrap();
                let title_tag = tag_list.get::<Title>();
                let artist_tag = tag_list.get::<Artist>();
                let album_tag = tag_list.get::<Album>();
                let datetime_tag = tag_list.get::<DateTime>();
                let genre_tag = tag_list.get::<Genre>();
                let track_number_tag = tag_list.get::<TrackNumber>();
                let album_volume_number_tag = tag_list.get::<AlbumVolumeNumber>();
                let album_artist_tag = tag_list.get::<AlbumArtist>();
                let lyrics_tag = tag_list.get::<Lyrics>();
                let values = (
                    path.eq(entry.path().strip_prefix(&collection.read().unwrap().path)?.to_str().unwrap()),
                    title.eq(title_tag.as_ref().as_str()),
                    artist.eq(artist_tag.as_ref().as_str()),
                    album.eq(album_tag.as_ref().as_str()),
                    year.eq(datetime_tag.map(|it| { it.get().year() })),
                    genre.eq(genre_tag.as_ref().as_str()),
                    track_number.eq(track_number_tag.map(|it| { it.get() as i32 })),
                    album_volume.eq(album_volume_number_tag.map(|it| { it.get() as i32 })),
                    album_artist.eq(album_artist_tag.as_ref().as_str()),
                    duration.eq(discoverer_info.duration().unwrap().nseconds() as i64),
                    lyrics.eq(lyrics_tag.as_ref().as_str()),
                    collection_id.eq(collection.read().unwrap().id),
                );
                insert_into(songs).values(values).on_conflict(path).do_update().set(values).execute(connection)?;
                let metadata = entry.metadata()?;
                Some(max(metadata.created()?, metadata.modified()?))
            } else {
                None
            }
        } else {
            None
        })
    }).try_fold(None, |prev, next| next.map(|ok| max(prev, ok)))?;
    if let Some(max_modified) = max_modified {
        let max_modified = max_modified.duration_since(UNIX_EPOCH)?.as_nanos() as i64;
        update(collections.find(collection.read().unwrap().id)).set(modified.eq(max_modified)).execute(connection)?;
        collection.write().unwrap().modified = Some(max_modified);
    }
    Ok(sender.send(ImportProgress::CollectionEnd(collection))?)
}

pub fn get_current_song(connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>)
    -> QueryResult<(Song, Config, Collection)> {
    songs.inner_join(config).inner_join(collections).get_result::<(Song, Config, Collection)>(connection)
}

pub fn get_current_album(artist_string: Option<impl AsRef<String>>, album_string: Option<impl AsRef<String>>,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>) -> Vec<(Song, Collection)> {
    let statement = songs.inner_join(collections).order_by((track_number, id)).into_boxed();
    let artist_filtered_statement = if let Some(artist_string) = &artist_string {
        statement.filter(artist.eq(artist_string.as_ref()))
    } else {
        statement.filter(artist.is_null())
    };
    if let Some(album_string) = &album_string {
        artist_filtered_statement.filter(album.eq(album_string.as_ref()))
    } else {
        artist_filtered_statement.filter(album.is_null())
    }.get_results::<(Song, Collection)>(connection).unwrap()
}
