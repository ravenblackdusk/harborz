#![allow(deprecated)]

use std::ops::{Deref};
use diesel::{ExpressionMethods, insert_into, RunQueryDsl};
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error::DatabaseError;
use gtk::FileChooserAction::SelectFolder;
use gtk::{glib, Button, FileChooserDialog, ResponseType};
use gtk::glib::{clone, MainContext};
use gtk::prelude::{DialogExtManual, FileExt, GtkWindowExt, FileChooserExt};
use log::warn;
use crate::CONNECTION;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

async fn add_directory_to_collection(dialog: FileChooserDialog) {
    if dialog.run_future().await == ResponseType::Ok {
        match insert_into(collections).values(
            path.eq(dialog.file().expect("ok dialog should have file")
                .path().expect("file should have path")
                .to_str().expect("path should be convertable to string"))
        ).execute(&mut CONNECTION.deref().get().expect("should be able to get connection from pool")) {
            Err(DatabaseError(UniqueViolation, _)) => warn!("directory already added to collection"),
            Err(error) => panic!("{error:?}"),
            _ => {}
        }
    }
    dialog.close();
}

pub fn choose_file(_button: &Button) {
    let dialog = FileChooserDialog::builder().title("my title").action(SelectFolder).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("ok", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog => async move {
        add_directory_to_collection(dialog).await;
    }));
}
