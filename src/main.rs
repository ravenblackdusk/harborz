mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;

use std::rc::Rc;
use diesel::migration;
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Application, ApplicationWindow, Button, glib, HeaderBar, Label, MenuButton, Popover, prelude};
use prelude::*;
use glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::common::gtk_box;
use crate::controls::media_controls;
use crate::db::get_connection;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_frame = collection::frame();
        let media_controls = media_controls();
        let bar = HeaderBar::builder().title_widget(&Label::builder().label("Music player").build()).build();
        let window = Rc::new(ApplicationWindow::builder().application(application).child(&media_controls).titlebar(&bar).build());
        let collection_button = Button::builder().label("Collection").build();
        collection_button.connect_clicked({
            let window = window.clone();
            move |_| { window.set_child(Some(&collection_frame)); }
        });
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let home = Button::builder().icon_name("go-home").build();
        home.connect_clicked({
            let window = window.clone();
            move |_| { window.set_child(Some(&media_controls)); }
        });
        bar.pack_start(&home);
        bar.pack_end(&MenuButton::builder().icon_name("open-menu-symbolic")
            .popover(&Popover::builder().child(&menu).build()).build());
        window.present();
    });
    Ok(application.run())
}
