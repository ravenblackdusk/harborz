#![allow(deprecated)]

use std::ops::{Deref};
use diesel::{ExpressionMethods, insert_into, RunQueryDsl};
use gtk::{glib, Button, FileChooserDialog, ResponseType};
use gtk::glib::{clone, MainContext};
use gtk::prelude::{DialogExtManual, FileExt, GtkWindowExt, FileChooserExt};
use crate::CONNECTION;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

pub fn choose_file(_button: &Button) {
    let dialog = FileChooserDialog::builder().title("my title").build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("ok", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog => async move {
        if dialog.run_future().await == ResponseType::Ok {
            if let Some(file) = dialog.file() {
                if let Some(path_buf) = file.path() {
                    if let Some(path_string) = path_buf.to_str() {
                        if let Ok(mut connection) = CONNECTION.deref().get() {
                            insert_into(collections).values(path.eq(path_string)).execute(&mut connection).ok();
                        }
                    }
                }
            }
        }
        dialog.close();
    }));
}
