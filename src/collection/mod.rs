#![allow(deprecated)]

mod grid;

use std::rc::Rc;
use anyhow::{anyhow, Result};
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl, Selectable};
use diesel::prelude::*;
use diesel::result::Error;
use gtk::*;
use prelude::*;
use gio::File;
use glib::{clone, MainContext};
use FileChooserAction::SelectFolder;
use Orientation::Vertical;
use crate::db::get_connection;
use crate::collection::grid::CollectionGrid;
use crate::common::gtk_box;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

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

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::collections)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct Collection {
    pub id: i32,
    pub path: String,
}

fn open_add_directories_to_collection_dialog(collection_grid: Rc<Grid>) {
    let dialog = FileChooserDialog::builder().title("choose collection directories")
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog, @weak collection_grid => async move {
        if dialog.run_future().await == ResponseType::Ok {
            add_directories_to_collection(&dialog, collection_grid).expect("should be able to add directories to collection");
        }
        dialog.close();
    }));
}

fn add_directories_to_collection(dialog: &FileChooserDialog, collection_grid: Rc<Grid>) -> Result<()> {
    dialog.files().iter::<File>().map(|file| { Some(file.ok()?.path()?.to_str()?.to_owned()) })
        .collect::<Option<Vec<_>>>().ok_or(anyhow!("error trying to get paths"))?.iter().filter_map(|path_string| {
        match insert_or_ignore_into(collections).values(path.eq(path_string)).get_result::<Collection>(&mut get_connection()) {
            Err(Error::NotFound) => None,
            result => Some(result),
        }
    }).map(|result| {
        Ok(collection_grid.clone().add(&result?, collections.count().first::<i64>(&mut get_connection())? as i32))
    }).collect::<Result<Vec<_>>>()?;
    Ok(())
}
