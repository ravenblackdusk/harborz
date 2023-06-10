mod choose_file;
mod schema;
mod models;

use std::env::var;
use diesel::{Connection, RunQueryDsl, SqliteConnection};
use dotenvy::dotenv;
use gtk::{Application, ApplicationWindow, Button, Grid};
use gtk::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt};
use gtk::traits::{ButtonExt, GridExt};
use crate::choose_file::choose;
use crate::models::Collection;
use crate::schema::collection::dsl::collections;

fn main() {
    dotenv().ok();
    let database_url = var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connection = &mut SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|app| {
        let grid = Grid::builder().build();
        let browse_button = Button::builder().label("browse").build();
        browse_button.connect_clicked(&choose);
        grid.attach(&browse_button, 0, 0, 1, 1);
        grid.attach(&Button::builder().label("Click me!").build(), 1, 0, 2, 1);
        ApplicationWindow::builder().application(app).title("music player").child(&grid).build()
            .present();
    });
    application.run();
}
