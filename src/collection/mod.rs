mod grid;
pub mod model;
pub mod song;
mod dialog;

use std::rc::Rc;
use std::time::UNIX_EPOCH;
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, update};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, Grid};
use gtk::prelude::*;
use gtk::gio::File;
use gtk::Orientation::Vertical;
use crate::collection::dialog::open_dialog;
use crate::db::get_connection;
use crate::collection::grid::CollectionGrid;
use crate::collection::model::Collection;
use crate::collection::song::import_songs;
use crate::common::gtk_box;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::{modified, path, row};

pub fn collection_box() -> gtk::Box {
    let collection_grid: Rc<Grid> = CollectionGrid::new();
    let browse_button = Button::builder().label("browse").build();
    let collection_box = gtk_box(Vertical);
    collection_box.append(&*collection_grid);
    collection_box.append(&browse_button);
    browse_button.connect_clicked(move |_| {
        open_dialog({
            let collection_grid = collection_grid.clone();
            move |files| {
                if let Some(files) = files {
                    for path_string in files.iter::<File>().map(|file| { Some(file.unwrap().path()?.to_str()?.to_owned()) })
                        .collect::<Option<Vec<_>>>().unwrap() {
                        get_connection().transaction(|connection| {
                            let max_row = collections.select(max(row)).get_result::<Option<i32>>(connection)?;
                            match insert_or_ignore_into(collections)
                                .values((path.eq(path_string), row.eq(max_row.unwrap_or(0) + 1)))
                                .get_result::<Collection>(connection) {
                                Err(Error::NotFound) => {}
                                Ok(collection) => {
                                    if let Some(system_time) = import_songs(&collection, connection) {
                                        update(collections.find(collection.id))
                                            .set(modified.eq(system_time.duration_since(UNIX_EPOCH)?.as_nanos() as i64))
                                            .execute(connection)?;
                                    }
                                    collection_grid.clone().add(&collection);
                                }
                                result => { result?; }
                            }
                            anyhow::Ok(())
                        }).unwrap();
                    }
                }
            }
        });
    });
    collection_box
}
