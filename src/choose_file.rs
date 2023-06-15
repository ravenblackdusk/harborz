#![allow(deprecated)]

use anyhow::{anyhow, Result};
use diesel::{ExpressionMethods, insert_or_ignore_into, QueryDsl, RunQueryDsl};
use diesel::result::Error;
use gtk::*;
use prelude::*;
use gio::File;
use glib::{clone, MainContext};
use FileChooserAction::SelectFolder;
use crate::db::get_connection;
use crate::models::Collection;
use crate::CollectionGrid;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

fn add_directory_to_collection(dialog: &FileChooserDialog, collection_grid: &Grid) -> Result<()> {
    dialog.files().iter::<File>().map(|file| { Some(file.ok()?.path()?.to_str()?.to_owned()) })
        .collect::<Option<Vec<_>>>().ok_or(anyhow!("error trying to get paths"))?.iter().filter_map(|path_string| {
        match insert_or_ignore_into(collections).values(path.eq(path_string)).get_result::<Collection>(&mut get_connection()) {
            Err(Error::NotFound) => None,
            result => Some(result),
        }
    }).map(|result| {
        Ok(collection_grid.add(&result?, collections.count().first::<i64>(&mut get_connection())? as i32))
    }).collect::<Result<Vec<_>>>()?;
    Ok(())
}

pub fn choose_file(collection_grid: &Grid) {
    let dialog = FileChooserDialog::builder().title("choose collection directories")
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog, @weak collection_grid => async move {
        if dialog.run_future().await == ResponseType::Ok {
            add_directory_to_collection(&dialog, &collection_grid).expect("should be able to add directories to collection");
        }
        dialog.close();
    }));
}
