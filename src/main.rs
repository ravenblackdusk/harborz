mod schema;
mod db;
mod collection;
mod controls;
mod common;

use diesel::migration;
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Application, ApplicationWindow, glib, prelude};
use prelude::*;
use glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::common::gtk_box;
use crate::db::get_connection;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let main_box = gtk_box(Vertical);
        main_box.append(&collection::frame());
        main_box.append(&controls::media_controls());
        ApplicationWindow::builder().application(application).title("music player").child(&main_box)
            .build().present();
    });
    Ok(application.run())
}
