#![allow(deprecated)]

use std::ops::{Deref};
use diesel::{ExpressionMethods, insert_or_ignore_into, RunQueryDsl};
use gtk::FileChooserAction::SelectFolder;
use gtk::{glib, Button, FileChooserDialog, ResponseType};
use gtk::gio::File;
use gtk::glib::{clone, MainContext};
use gtk::prelude::{DialogExtManual, FileExt, GtkWindowExt, FileChooserExt, ListModelExtManual};
use crate::CONNECTION;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

async fn add_directory_to_collection(dialog: FileChooserDialog) {
    if dialog.run_future().await == ResponseType::Ok {
        insert_or_ignore_into(collections).values(dialog.files().iter::<File>().map(|file| {
            path.eq(file.expect("ok dialog should have file").path().expect("file should have path")
                .to_str().expect("path should be convertable to string").to_owned())
        }).collect::<Vec<_>>())
            .execute(&mut CONNECTION.deref().get().expect("should be able to get connection from pool"))
            .expect("should be able to insert collections");
    }
    dialog.close();
}

pub fn choose_file(_button: &Button) {
    let dialog = FileChooserDialog::builder().title("my title").action(SelectFolder)
        .select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("ok", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog => async move {
        add_directory_to_collection(dialog).await;
    }));
}
