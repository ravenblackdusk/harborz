#![allow(deprecated)]

use std::collections::HashSet;
use anyhow::{anyhow, Result};
use diesel::{ExpressionMethods, insert_or_ignore_into, RunQueryDsl};
use gtk::*;
use prelude::*;
use gio::File;
use glib::{clone, MainContext, Object};
use FileChooserAction::SelectFolder;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;

async fn add_directory_to_collection(dialog: &FileChooserDialog, collection_box: &Box) -> Result<()> {
    if dialog.run_future().await == ResponseType::Ok {
        let paths = dialog.files().iter::<File>().map(|file| { Some(file.ok()?.path()?.to_str()?.to_owned()) })
            .collect::<Option<Vec<_>>>().ok_or(anyhow!("error trying to get paths"))?;
        insert_or_ignore_into(collections)
            .values(paths.iter().map(|path_string| { path.eq(path_string) }).collect::<Vec<_>>())
            .execute(&mut get_connection())?;
        let existing_paths = collection_box.observe_children().iter::<Object>().map(|child| {
            Ok(child?.downcast::<Label>().map_err(|object| { anyhow!("error downcasting {:?}", object) })?.label().to_string())
        }).collect::<Result<HashSet<_>>>()?;
        for added_path in paths.into_iter().filter(|added_path| { !existing_paths.contains(added_path) }) {
            collection_box.append(&Label::builder().label(added_path).build())
        }
    }
    Ok(dialog.close())
}

pub fn choose_file(collection_box: &Box) {
    let dialog = FileChooserDialog::builder().title("choose collection directories")
        .action(SelectFolder).select_multiple(true).build();
    dialog.add_buttons(&[("cancel", ResponseType::Cancel), ("choose", ResponseType::Ok)]);
    MainContext::default().spawn_local(clone!(@weak dialog, @weak collection_box => async move {
        add_directory_to_collection(&dialog, &collection_box).await.expect("should be able to add directories to collection");
    }));
}
