#![allow(deprecated)]

use gtk::{glib, Button, FileChooserDialog, ResponseType};
use gtk::glib::{clone, MainContext};
use gtk::prelude::{DialogExtManual, GtkWindowExt};

pub static CHOOSE_FILE: fn(&Button) = |_| {
    let dialog = FileChooserDialog::builder().title("my title").build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("ok", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog => async move {
        let a = dialog.run_future().await;
        match a {
            ResponseType::Ok => println!("ok"),
            ResponseType::Cancel => println!("cancel"),
            _ => {}
        }
        dialog.close();
    }));
};
