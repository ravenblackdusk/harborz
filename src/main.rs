mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;

use diesel::migration;
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Application, ApplicationWindow, Button, glib, HeaderBar, Label, MenuButton, Popover, prelude};
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
        let bar = HeaderBar::builder().title_widget(&Label::builder().label("music player").build()).build();
        let menu = gtk_box(Vertical);
        let collection_button = Button::builder().label("Collection").build();
        menu.append(&collection_button);
        let back = Button::builder().icon_name("go-home").build();
        bar.pack_start(&back);
        bar.pack_end(&MenuButton::builder().icon_name("open-menu-symbolic")
            .popover(&Popover::builder().child(&menu).build()).build());
        ApplicationWindow::builder().application(application).child(&main_box).titlebar(&bar).build().present();
    });
    Ok(application.run())
}
