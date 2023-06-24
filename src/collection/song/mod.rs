use std::time::SystemTime;
use diesel::{insert_or_ignore_into, Queryable, RunQueryDsl, Selectable, ExpressionMethods, SqliteConnection};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::result::Error;
use walkdir::WalkDir;
use crate::collection::model::Collection;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::{collection_id, path};

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
        Ok(if let Err(Error::NotFound) = insert_or_ignore_into(songs).values(
            (path.eq(entry.path().strip_prefix(&collection.path)?.to_str().unwrap()),
                collection_id.eq(collection.id))
        ).execute(connection) {
            None
        } else {
            Some(entry.metadata()?.modified()?)
        })
    }).filter_map(|result| { result.unwrap() }).max()
}
