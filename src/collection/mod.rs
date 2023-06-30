mod r#box;
pub mod model;
pub mod song;
mod dialog;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, UNIX_EPOCH};
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, update};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, ProgressBar};
use gtk::prelude::*;
use gtk::gio::File;
use gtk::glib::timeout_add_local;
use gtk::Orientation::Vertical;
use crate::collection::dialog::open_dialog;
use crate::db::get_connection;
use crate::collection::r#box::CollectionBox;
use crate::collection::model::Collection;
use crate::collection::song::{import_songs, ImportProgress};
use crate::common::gtk_box;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::{modified, path, row};

pub fn add_collection_box() -> gtk::Box {
    let add_collection_box = gtk_box(Vertical);
    let collection_box: gtk::Box = CollectionBox::new();
    let browse_button = Button::builder().label("browse").build();
    add_collection_box.append(&collection_box);
    add_collection_box.append(&browse_button);
    browse_button.connect_clicked({
        let collection_box = collection_box.clone();
        move |_| {
            open_dialog({
                let collection_box = collection_box.clone();
                move |files| {
                    if let Some(files) = files {
                        let (sender, receiver) = channel::<ImportProgress>();
                        let mut last_id: Option<i32> = None;
                        let mut progress_bar_map = HashMap::new();
                        timeout_add_local(Duration::from_millis(200), {
                            let collection_box = collection_box.clone();
                            move || {
                                Continue(if let Ok(import_progress) = receiver.try_recv() {
                                    match import_progress {
                                        ImportProgress::CollectionStart(id) => {
                                            last_id = Some(id);
                                            let progress_bar = ProgressBar::builder().hexpand(true).build();
                                            collection_box.append(&progress_bar);
                                            progress_bar_map.insert(id, progress_bar);
                                            true
                                        }
                                        ImportProgress::Pulse => {
                                            progress_bar_map[&last_id.unwrap()].pulse();
                                            true
                                        }
                                        ImportProgress::Fraction(fraction) => {
                                            progress_bar_map[&last_id.unwrap()].set_fraction(fraction);
                                            true
                                        }
                                        ImportProgress::CollectionEnd(id, collection_path) => {
                                            collection_box.remove(&progress_bar_map[&last_id.unwrap()]);
                                            collection_box.add(id, &collection_path);
                                            true
                                        }
                                        ImportProgress::End => false
                                    }
                                } else { false })
                            }
                        });
                        let paths = files.iter::<File>().map(|file| { Some(file.unwrap().path()?.to_str()?.to_owned()) })
                            .collect::<Option<Vec<_>>>().unwrap();
                        thread::spawn({
                            let sender = sender.clone();
                            move || {
                                for path_string in paths {
                                    get_connection().transaction({
                                        let sender = sender.clone();
                                        |connection| {
                                            let max_row = collections.select(max(row)).get_result::<Option<i32>>(connection)?;
                                            match insert_or_ignore_into(collections)
                                                .values((path.eq(path_string), row.eq(max_row.unwrap_or(0) + 1)))
                                                .get_result::<Collection>(connection) {
                                                Err(Error::NotFound) => {}
                                                Ok(collection) => {
                                                    if let Some(system_time) = import_songs(&collection, sender, connection) {
                                                        update(collections.find(collection.id))
                                                            .set(modified.eq(system_time.duration_since(UNIX_EPOCH)?.as_nanos() as i64))
                                                            .execute(connection)?;
                                                    }
                                                }
                                                result => { result?; }
                                            }
                                            anyhow::Ok(())
                                        }
                                    }).unwrap();
                                }
                                sender.send(ImportProgress::End).unwrap();
                            }
                        });
                    }
                }
            });
        }
    });
    add_collection_box
}
