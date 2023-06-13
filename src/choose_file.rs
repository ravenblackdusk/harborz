#![allow(deprecated)]

use std::collections::HashSet;
use diesel::{ExpressionMethods, insert_or_ignore_into, RunQueryDsl};
use gtk::FileChooserAction::SelectFolder;
use gtk::{glib, FileChooserDialog, ResponseType, Label};
use gtk::gio::File;
use gtk::glib::{clone, MainContext, Object};
use gtk::prelude::*;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

async fn add_directory_to_collection(dialog: &FileChooserDialog, collection_box: &gtk::Box) {
    if dialog.run_future().await == ResponseType::Ok {
        let paths = dialog.files().iter::<File>().map(|file| {
            file.expect("ok dialog should have file").path().expect("file should have path")
                .to_str().expect("path should be convertable to string").to_owned()
        }).collect::<Vec<_>>();
        insert_or_ignore_into(collections)
            .values(paths.iter().map(|path_string| { path.eq(path_string) }).collect::<Vec<_>>())
            .execute(&mut get_connection()).expect("should be able to insert collections");
        let existing_paths = collection_box.observe_children().iter::<Object>().map(|child| {
            child.expect("should be able to get child").downcast::<Label>()
                .expect("should be a label").label().to_string()
        }).collect::<HashSet<_>>();
        for added_path in paths.into_iter().filter(|added_path| { !existing_paths.contains(added_path) }) {
            collection_box.append(&Label::builder().label(added_path).build())
        }
    }
    dialog.close();
}

pub fn choose_file(collection_box: &gtk::Box) {
    let dialog = FileChooserDialog::builder().title("choose collection directories")
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog, @strong collection_box => async move {
        add_directory_to_collection(&dialog, &collection_box).await;
    }));
}
