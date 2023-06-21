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
        let frame1 = Box::new(collection::frame());
        let frame2 = Box::new(media_controls());
        let bar = HeaderBar::builder().title_widget(&Label::builder().label("music player").build()).build();
        let window = Rc::new(ApplicationWindow::builder().application(application).child(&*frame2).titlebar(&bar).build());
        let collection_button = Button::builder().label("Collection").build();
        collection_button.connect_clicked({
            let window = window.clone();
            move |_| { window.set_child(Some(&*frame1)); }
        });
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let home = Button::builder().icon_name("go-home").build();
        home.connect_clicked({
            let window = window.clone();
            move |_| { window.set_child(Some(&*frame2)); }
        });
        bar.pack_start(&home);
        bar.pack_end(&MenuButton::builder().icon_name("open-menu-symbolic")
            .popover(&Popover::builder().child(&menu).build()).build());
        window.present();
    });
    Ok(application.run())
}
