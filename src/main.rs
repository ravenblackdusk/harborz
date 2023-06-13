mod choose_file;
mod schema;
mod models;
mod db;

use diesel::{RunQueryDsl};
use dotenvy::dotenv;
use gtk::{Application, ApplicationWindow, Button, Grid, Label};
use gtk::glib;
use glib::clone;
use gtk::prelude::*;
use gtk::traits::{BoxExt, ButtonExt, GridExt};
use gtk::Orientation::Vertical;
use crate::choose_file::choose_file;
use crate::db::get_connection;
use crate::models::Collection;
use crate::schema::collections::dsl::collections;

fn main() {
    dotenv().ok();
    std_logger::Config::logfmt().init();
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let grid = Grid::builder().build();
        let window = ApplicationWindow::builder().application(application).title("music player")
            .child(&grid).build();
        let browse_button = Button::builder().label("browse").build();
        let collection_box = gtk::Box::builder().orientation(Vertical).build();
        browse_button.connect_clicked(clone!(@weak collection_box => move |_| { choose_file(&collection_box); }));
        for collection in collections.load::<Collection>(&mut get_connection()).expect("should be able to get collections from db") {
            collection_box.append(&Label::builder().label(collection.path).build());
        }
        grid.attach(&collection_box, 0, 0, 1, 1);
        grid.attach(&browse_button, 0, 1, 1, 1);
        grid.attach(&Button::builder().label("Click me!").build(), 1, 1, 1, 1);
        window.present();
    });
    application.run();
}
