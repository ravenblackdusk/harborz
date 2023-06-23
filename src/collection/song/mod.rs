use std::thread;
use diesel::{insert_or_ignore_into, Queryable, RunQueryDsl, Selectable, ExpressionMethods, Connection};
use walkdir::WalkDir;
use crate::collection::model::Collection;
use crate::db::get_connection;
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

pub(in crate::collection) fn import_songs(collections: Vec<Collection>) {
    if !collections.is_empty() {
        thread::spawn(|| {
            for collection in collections {
                get_connection().transaction(|connection| {
                    WalkDir::new(&collection.path).into_iter().filter_map(|entry_result| {
                        let entry = entry_result.unwrap();
                        if entry.file_type().is_file() {
                            insert_or_ignore_into(songs).values(
                                (path.eq(entry.path().strip_prefix(&collection.path).unwrap().to_str().unwrap()),
                                    collection_id.eq(collection.id))
                            ).execute(connection).unwrap();
                            Some(entry.metadata().unwrap().modified().unwrap())
                        } else {
                            None
                        }
                    }).max();
                    Ok::<_, anyhow::Error>(())
                })?;
            }
            Ok::<_, anyhow::Error>(())
        });
    }
}
