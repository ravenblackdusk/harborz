mod choose_file;
mod schema;
mod models;
mod db;

use diesel::{migration, delete, QueryDsl, RunQueryDsl};
use migration::Result;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use gtk::{prelude, glib, Button, Label, Application, Grid, ApplicationWindow};
use prelude::*;
use glib::{clone, ExitCode};
use crate::choose_file::choose_file;
use crate::db::get_connection;
use crate::models::Collection;
use crate::schema::collections::dsl::collections;

trait CollectionGrid {
    fn add(&self, collection: &Collection, row: i32);
}

impl CollectionGrid for Grid {
    fn add(&self, collection: &Collection, row: i32) {
        let remove_button = Button::builder().label("-").build();
        let id = collection.id;
        remove_button.connect_clicked(clone!(@weak self as borrowed_self => move |_| {
            delete(collections.find(id)).execute(&mut get_connection())
                .expect("should be able to delete collection");
            borrowed_self.remove_row(row);
        }));
        self.attach(&Label::builder().label(&collection.path).build(), 0, row, 1, 1);
        self.attach(&remove_button, 1, row, 1, 1);
    }
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_grid = Grid::builder().row_spacing(4).column_spacing(4).build();
        for (i, collection) in collections.load::<Collection>(&mut get_connection())
            .expect("should be able to get collections").into_iter().enumerate() {
            collection_grid.add(&collection, i as i32);
        }
        let browse_button = Button::builder().label("browse").build();
        browse_button.connect_clicked(clone!(@weak collection_grid => move |_| { choose_file(&collection_grid); }));
        let grid = Grid::builder().row_spacing(4).column_spacing(4).margin_start(4).margin_end(4)
            .margin_bottom(4).margin_top(4).build();
        grid.attach(&collection_grid, 0, 0, 1, 1);
        grid.attach(&browse_button, 0, 1, 1, 1);
        ApplicationWindow::builder().application(application).title("music player").child(&grid)
            .build().present();
    });
    Ok(application.run())
}
