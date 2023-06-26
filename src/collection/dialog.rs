#![allow(deprecated)]

use gtk::FileChooserAction::SelectFolder;
use gtk::{FileChooserDialog, ResponseType};
use gtk::gio::ListModel;
use gtk::glib::MainContext;
use gtk::prelude::{DialogExtManual, FileChooserExt, GtkWindowExt};

pub(in crate::collection) fn open_dialog<F: Fn(Option<ListModel>) + 'static>(do_with_files: F) {
    let dialog = FileChooserDialog::builder().title("Choose collection directories").use_header_bar(1)
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("Cancel", ResponseType::Cancel), ("Choose", ResponseType::Ok)]);
    MainContext::default().spawn_local({
        async move {
            let files = if dialog.run_future().await == ResponseType::Ok {
                Some(dialog.files())
            } else {
                None
            };
            dialog.close();
            do_with_files(files);
        }
    });
}
