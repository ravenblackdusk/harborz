use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::time::Duration;
use TryRecvError::{Disconnected, Empty};
use adw::gio::{Cancellable, File};
use adw::glib::{ControlFlow::*, timeout_add_local};
use adw::prelude::*;
use async_std::task;
use diesel::{delete, ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl};
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, FileDialog, Label, ProgressBar};
use gtk::Orientation::{Horizontal, Vertical};
use log::error;
use crate::body::collection::model::Collection;
use crate::common::{gtk_box, StyledLabelBuilder, StyledWidget};
use crate::common::constant::DESTRUCTIVE_ACTION;
use crate::common::state::State;
use crate::common::util::PathString;
use crate::db::get_connection;
use crate::schema::bodies::dsl::bodies;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::song::{import_songs, ImportProgress};

pub mod model;
pub mod button;
pub mod body;

fn handle_progress<F: Fn(Arc<RwLock<Collection>>) + 'static>(collections_box: &gtk::Box, on_collection_end: F)
    -> Sender<ImportProgress> {
    let (sender, receiver) = channel::<ImportProgress>();
    let progress_bar = ProgressBar::builder().hexpand(true).build();
    timeout_add_local(Duration::from_millis(500), {
        let collections_box = collections_box.clone();
        move || {
            let mut last_fraction = None;
            loop {
                match receiver.try_recv() {
                    Err(Empty) => { break; }
                    Err(Disconnected) => { return Break; }
                    Ok(ImportProgress::CollectionStart) => {
                        progress_bar.set_fraction(0.0);
                        collections_box.append(&progress_bar);
                    }
                    Ok(ImportProgress::CollectionEnd(collection)) => {
                        collections_box.remove(&progress_bar);
                        on_collection_end(collection);
                        last_fraction = None;
                    }
                    Ok(ImportProgress::Fraction(fraction)) => { last_fraction = Some(fraction); }
                }
            }
            if let Some(fraction) = last_fraction { progress_bar.set_fraction(fraction); }
            Continue
        }
    });
    sender
}

fn add(collections_box: &gtk::Box, collection: Arc<RwLock<Collection>>, state: Rc<State>) {
    let collection_box = gtk_box(Horizontal);
    collections_box.append(&collection_box);
    collection_box.append(&Label::builder()
        .label(collection.read().unwrap().path.to_path().file_name().unwrap().to_str().unwrap()).margin_ellipsized(4)
        .build()
    );
    let sync_button = Button::builder().icon_name("view-refresh").build();
    collection_box.append(&sync_button);
    let id = collection.read().unwrap().id;
    sync_button.connect_clicked({
        let collections_box = collections_box.clone();
        move |_| {
            let sender = handle_progress(&collections_box, |_| {});
            task::spawn({
                let collection = collection.clone();
                async move {
                    get_connection().transaction(|connection| import_songs(collection, sender, connection))
                        .unwrap();
                }
            });
        }
    });
    let remove_button = Button::builder().icon_name("list-remove").build().with_css_class(DESTRUCTIVE_ACTION);
    collection_box.append(&remove_button);
    remove_button.connect_clicked({
        let collections_box = collections_box.clone();
        move |_| {
            get_connection().transaction(|connection| {
                delete(collections.find(id)).execute(connection)?;
                delete(bodies).execute(connection)
            }).unwrap();
            state.history.borrow_mut().clear();
            collections_box.remove(&collection_box);
        }
    });
}

fn add_collection_box(state: Rc<State>) -> gtk::Box {
    let add_collection_box = gtk_box(Vertical);
    let collections_box = gtk_box(Vertical);
    add_collection_box.append(&collections_box);
    for collection in collections.load::<Collection>(&mut get_connection()).unwrap() {
        add(&collections_box, Arc::new(RwLock::new(collection)), state.clone());
    }
    let browse_button = Button::builder().label("Browse").build().suggested_action();
    add_collection_box.append(&browse_button);
    browse_button.connect_clicked({
        move |_| {
            FileDialog::builder().title("Collection directories").accept_label("Choose").build()
                .select_multiple_folders(Some(&state.window), Cancellable::NONE, {
                    let collections_box = collections_box.clone();
                    let state = state.clone();
                    move |files| {
                        match files {
                            Ok(files) => {
                                if let Some(files) = files {
                                    delete(bodies).execute(&mut get_connection()).unwrap();
                                    state.history.borrow_mut().clear();
                                    let paths = files.into_iter()
                                        .map(|file| { file.unwrap().downcast::<File>().unwrap().path().unwrap() })
                                        .collect::<Vec<_>>();
                                    let sender = handle_progress(&collections_box, {
                                        let collections_box = collections_box.clone();
                                        move |collection| { add(&collections_box, collection, state.clone()); }
                                    });
                                    task::spawn(async move {
                                        for path_buf in paths {
                                            get_connection().transaction(|connection| {
                                                anyhow::Ok(match insert_or_ignore_into(collections)
                                                    .values(path.eq(path_buf.to_str().unwrap()))
                                                    .get_result::<Collection>(connection) {
                                                    Err(Error::NotFound) => {}
                                                    Ok(collection) => {
                                                        import_songs(Arc::new(RwLock::new(collection)), sender.clone(),
                                                            connection)?
                                                    }
                                                    result => { result?; }
                                                })
                                            }).unwrap();
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
