mod choose_file;
mod schema;
mod models;

use std::env::var;
use diesel::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use gtk::{Application, ApplicationWindow, Button, Grid};
use gtk::prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt};
use gtk::traits::{ButtonExt, GridExt};
use once_cell::sync::Lazy;
use crate::choose_file::choose_file;

const DATABASE_URL: &'static str = "DATABASE_URL";
static CONNECTION: Lazy<Pool<ConnectionManager<SqliteConnection>>> = Lazy::new(|| {
    let database_url = var(DATABASE_URL).expect(format!("{} must be set", DATABASE_URL).as_str());
    Pool::builder().test_on_check_out(true).build(ConnectionManager::<SqliteConnection>::new(database_url))
        .expect("Could not build connection pool")
});

fn main() {
    dotenv().ok();
    std_logger::Config::logfmt().init();
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|app| {
        let grid = Grid::builder().build();
        let browse_button = Button::builder().label("browse").build();
        browse_button.connect_clicked(&choose_file);
        grid.attach(&browse_button, 0, 0, 1, 1);
        grid.attach(&Button::builder().label("Click me!").build(), 1, 0, 2, 1);
        ApplicationWindow::builder().application(app).title("music player").child(&grid).build()
            .present();
    });
    application.run();
}
