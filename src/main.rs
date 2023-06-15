mod schema;
mod db;
mod collection;

use diesel::migration;
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Application, ApplicationWindow, glib, prelude};
use prelude::*;
use glib::ExitCode;
use db::MIGRATIONS;
use crate::db::get_connection;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        ApplicationWindow::builder().application(application).title("music player")
            .child(&collection::frame()).build().present();
    });
    Ok(application.run())
}
