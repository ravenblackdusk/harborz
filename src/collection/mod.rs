#![allow(deprecated)]

mod grid;
mod model;

use std::rc::Rc;
use anyhow::{anyhow, Result};
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
        open_add_directories_to_collection_dialog(collection_grid.clone());
    });
    Frame::builder().child(&collection_box).build()
}

fn open_add_directories_to_collection_dialog(collection_grid: Rc<Grid>) {
    let dialog = FileChooserDialog::builder().title("choose collection directories")
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
    MainContext::default().spawn_local(async move {
        if dialog.run_future().await == ResponseType::Ok {
            add_directories_to_collection(&dialog, collection_grid).expect("should be able to add directories to collection");
        }
        dialog.close();
    });
}

fn add_directories_to_collection(dialog: &FileChooserDialog, collection_grid: Rc<Grid>) -> Result<()> {
    dialog.files().iter::<File>().map(|file| { Some(file.ok()?.path()?.to_str()?.to_owned()) })
        .collect::<Option<Vec<_>>>().ok_or(anyhow!("error trying to get paths"))?.iter().filter_map(|path_string| {
        match get_connection().transaction(|connection| {
            let max_row = collections.select(max(row)).get_result::<Option<i32>>(connection)?;
            insert_or_ignore_into(collections).values((path.eq(path_string), row.eq(max_row.unwrap_or(0) + 1)))
                .get_result::<Collection>(connection)
        }) {
            Err(Error::NotFound) => None,
            result => Some(result),
        }
    }).map(|result| { Ok(collection_grid.clone().add(&result?)) }).collect::<Result<Vec<_>>>()?;
    Ok(())
}
