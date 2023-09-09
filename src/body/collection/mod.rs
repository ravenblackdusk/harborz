use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};
use TryRecvError::{Disconnected, Empty};
use adw::gio::{Cancellable, File};
use adw::glib::{ControlFlow::*, timeout_add_local};
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, update};
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, FileDialog, ProgressBar};
use gtk::Orientation::Vertical;
use log::error;
use crate::body::collection::model::Collection;
use crate::body::collection::r#box::CollectionBox;
use crate::common::{gtk_box, StyledWidget};
use crate::common::state::State;
use crate::db::get_connection;
use crate::schema::bodies::dsl::bodies;
use crate::schema::collections::{modified, path};
use crate::schema::collections::dsl::collections;
use crate::song::{import_songs, ImportProgress};

mod r#box;
pub mod model;
pub mod button;
pub mod body;

fn add_collection_box(state: Rc<State>) -> gtk::Box {
    let add_collection_box = gtk_box(Vertical);
    let collection_box: gtk::Box = CollectionBox::new(state.history.clone());
    add_collection_box.append(&collection_box);
    let browse_button = Button::builder().label("Browse").build().suggested_action();
    add_collection_box.append(&browse_button);
    browse_button.connect_clicked({
        move |_| {
            FileDialog::builder().title("Collection directories").accept_label("Choose").build()
                .select_multiple_folders(Some(&state.window), Cancellable::NONE, {
                    let collection_box = collection_box.clone();
                    let state = state.clone();
                    move |files| {
                        match files {
                            Ok(files) => {
                                if let Some(files) = files {
                                    delete(bodies).execute(&mut get_connection()).unwrap();
                                    state.history.borrow_mut().clear();
                                    let (sender, receiver) = channel::<ImportProgress>();
                                    let mut last_id: Option<i32> = None;
                                    let mut progress_bar_map = HashMap::new();
                                    timeout_add_local(Duration::from_millis(500), {
                                        let collection_box = collection_box.clone();
                                        move || {
                                            match receiver.try_recv() {
                                                Err(Empty) => Continue,
                                                Err(Disconnected) => Break,
                                                Ok(import_progress) => {
                                                    match import_progress {
                                                        ImportProgress::CollectionStart(id) => {
                                                            last_id = Some(id);
                                                            let progress_bar
                                                                = ProgressBar::builder().hexpand(true).build();
                                                            collection_box.append(&progress_bar);
                                                            progress_bar_map.insert(id, progress_bar);
                                                            Continue
                                                        }
                                                        ImportProgress::Fraction(fraction) => {
                                                            progress_bar_map[&last_id.unwrap()].set_fraction(fraction);
                                                            Continue
                                                        }
                                                        ImportProgress::CollectionEnd(id, collection_path) => {
                                                            collection_box.remove(&progress_bar_map[&last_id.unwrap()]);
                                                            collection_box
                                                                .add(id, &collection_path, state.history.clone());
                                                            Continue
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });
                                    let paths = files.iter::<File>().map(|file| {
                                        Some(file.unwrap().path()?.to_str()?.to_owned())
                                    }).collect::<Option<Vec<_>>>().unwrap();
                                    thread::spawn({
                                        let sender = sender.clone();
                                        move || {
                                            for path_string in paths {
                                                get_connection().transaction({
                                                    let sender = sender.clone();
                                                    |connection| {
                                                        match insert_or_ignore_into(collections)
                                                            .values(path.eq(path_string))
                                                            .get_result::<Collection>(connection) {
                                                            Err(Error::NotFound) => {}
                                                            Ok(collection) => {
                                                                if let Some(system_time)
                                                                    = import_songs(&collection, sender, connection) {
                                                                    update(collections.find(collection.id)).set(
                                                                        modified.eq(system_time
                                                                            .duration_since(UNIX_EPOCH)
                                                                            ?.as_nanos() as i64
                                                                        )).execute(connection)?;
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
                            Err(error) => { error!("error Choosing Collection directory [{error}]"); }
                        }
                    }
                });
        }
    });
    add_collection_box
}
