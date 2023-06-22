#![allow(deprecated)]

mod grid;
mod model;

use std::rc::Rc;
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::result::Error;
use gtk::{Button, FileChooserDialog, Frame, Grid, ResponseType};
use gtk::prelude::*;
use gtk::gio::File;
use gtk::glib::MainContext;
use gtk::FileChooserAction::SelectFolder;
use gtk::Orientation::Vertical;
use crate::db::get_connection;
use crate::collection::grid::CollectionGrid;
use crate::collection::model::Collection;
use crate::common::gtk_box;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::{path, row};

pub fn frame() -> Frame {
    let collection_grid: Rc<Grid> = CollectionGrid::new();
    let browse_button = Button::builder().label("browse").build();
    let collection_box = gtk_box(Vertical);
    collection_box.append(&*collection_grid);
    collection_box.append(&browse_button);
    browse_button.connect_clicked(move |_| {
        let dialog = FileChooserDialog::builder().title("choose collection directories")
            .action(SelectFolder).select_multiple(true).build();
        dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
        MainContext::default().spawn_local({
            let collection_grid = collection_grid.clone();
            async move {
                if dialog.run_future().await == ResponseType::Ok {
                    dialog.files().iter::<File>().map(|file| { Some(file.ok()?.path()?.to_str()?.to_owned()) })
                        .collect::<Option<Vec<_>>>().unwrap().iter().filter_map(|path_string| {
                        match get_connection().transaction(|connection| {
                            let max_row = collections.select(max(row)).get_result::<Option<i32>>(connection)?;
                            insert_or_ignore_into(collections).values((path.eq(path_string), row.eq(max_row.unwrap_or(0) + 1)))
                                .get_result::<Collection>(connection)
                        }) {
                            Err(Error::NotFound) => None,
                            result => Some(result),
                        }
                    }).for_each(|result| { collection_grid.clone().add(&result.unwrap()); });
                }
                dialog.close();
            }
        });
    });
    Frame::builder().child(&collection_box).build()
}
