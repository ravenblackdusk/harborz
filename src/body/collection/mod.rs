use std::collections::HashMap;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};
use TryRecvError::{Disconnected, Empty};
use adw::ApplicationWindow;
use adw::prelude::*;
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, update};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, FileDialog, ProgressBar};
use gtk::gio::{Cancellable, File};
use gtk::glib::timeout_add_local;
use gtk::Orientation::Vertical;
use crate::body::collection::model::Collection;
use crate::body::collection::r#box::CollectionBox;
use crate::common::gtk_box;
use crate::db::get_connection;
use crate::schema::collections::{modified, path, row};
use crate::schema::collections::dsl::collections;
use crate::song::{import_songs, ImportProgress};

mod r#box;
pub mod model;

pub(in crate::body) fn add_collection_box(window: &ApplicationWindow) -> gtk::Box {
    let add_collection_box = gtk_box(Vertical);
    let collection_box: gtk::Box = CollectionBox::new();
    let browse_button = Button::builder().label("browse").build();
    add_collection_box.append(&collection_box);
    add_collection_box.append(&browse_button);
    browse_button.connect_clicked({
        let window = window.clone();
        move |_| {
            FileDialog::builder().title("Collection directories").accept_label("Choose").build()
                .open_multiple(Some(&window), Cancellable::NONE, {
                    let collection_box = collection_box.clone();
                    move |files| {
                        if let Ok(files) = files {
                            let (sender, receiver) = channel::<ImportProgress>();
                            let mut last_id: Option<i32> = None;
                            let mut progress_bar_map = HashMap::new();
                            timeout_add_local(Duration::from_millis(200), {
                                let collection_box = collection_box.clone();
                                move || {
                                    Continue(match receiver.try_recv() {
                                        Err(Empty) => { true }
                                        Err(Disconnected) => { false }
                                        Ok(import_progress) => {
                                            match import_progress {
                                                ImportProgress::CollectionStart(id) => {
                                                    last_id = Some(id);
                                                    let progress_bar = ProgressBar::builder().hexpand(true).build();
                                                    collection_box.append(&progress_bar);
                                                    progress_bar_map.insert(id, progress_bar);
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
                                            }
                                        }
                                    })
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
                                }
                            });
                        }
                    }
                });
        }
    });
    add_collection_box
}