#![allow(deprecated)]

use gtk::{glib, Button, FileChooserDialog, ResponseType};
use gtk::glib::{clone, MainContext};
use gtk::prelude::{DialogExtManual, FileExt, GtkWindowExt, FileChooserExt};

pub fn choose(_button: &Button) {
    let dialog = FileChooserDialog::builder().title("my title").build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("ok", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog => async move {
        if dialog.run_future().await == ResponseType::Ok {
            println!("{:?}", dialog.file().expect("no file").path())
        }
        dialog.close();
    }));
}
