use std::time::{Duration, SystemTime};
use diesel::{insert_or_ignore_into, Queryable, RunQueryDsl, Selectable, ExpressionMethods, SqliteConnection};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use gstreamer::{Element, ElementFactory, MessageType};
use gstreamer::MessageView::Tag;
use gstreamer::prelude::{ElementExt, ObjectExt};
use gstreamer::State::Paused;
use gstreamer::tags::{Album, AlbumArtist, Artist, DateTime, Genre, Title, TrackNumber};
use once_cell::sync::Lazy;
use walkdir::WalkDir;
use crate::collection::model::Collection;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::songs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct Song {
    pub id: i32,
    pub path: String,
    pub collection_id: i32,
}

pub(in crate::collection) fn import_songs(collection: &Collection,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>) -> Option<SystemTime> {
    WalkDir::new(&collection.path).into_iter().filter_map(|entry_result| {
        let entry = entry_result.unwrap();
        if entry.file_type().is_file() { Some(entry) } else { None }
    }).map(|entry| -> anyhow::Result<_> {
        let element = ElementFactory::make("playbin").build().unwrap();
        element.set_property("uri", format!("file:{}", entry.path().to_str().unwrap()));
        element.set_state(Paused)?;
        if let Tag(tag) = element.bus().unwrap().timed_pop_filtered(None, &[MessageType::Tag]).unwrap().view() {
            let tag_list = tag.tags();
            let tag_list_ref = tag_list.as_ref();
            let x = entry.path().strip_prefix(&collection.path)?.to_str().unwrap();
            let value = tag_list_ref.get::<Title>().unwrap();
            let x1 = value.get();
            let value1 = tag_list_ref.get::<Artist>().unwrap();
            let x2 = value1.get();
            let value2 = tag_list_ref.get::<Album>().unwrap();
            let x3 = value2.get();
            let option = tag_list_ref.get::<DateTime>().and_then(|date_time| { date_time.get().microsecond() }).map(|date_time| { Duration::from_micros(date_time as u64).as_nanos() as i64 });
            let value3 = tag_list_ref.get::<Genre>().unwrap();
            let x4 = value3.get();
            let i = tag_list_ref.get::<TrackNumber>().unwrap().get() as i32;
            let option1 = tag_list_ref.get::<AlbumArtist>().map(|it| { it.get().to_owned() });
            Ok(if let Err(Error::NotFound) = insert_or_ignore_into(songs).values((
                path.eq(x),
                title.eq(x1),
                artist.eq(x2),
                album.eq(x3),
                datetime.eq(option),
                genre.eq(x4),
                track_number.eq(i),
                album_artist.eq(option1),
                collection_id.eq(collection.id),
            )).execute(connection) {
                None
            } else {
                Some(entry.metadata()?.modified()?)
            })
        } else {
            panic!()
        }
    }).filter_map(|result| { result.unwrap() }).max()
}
