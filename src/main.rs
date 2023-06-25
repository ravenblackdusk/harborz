mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;
mod home;

use std::rc::Rc;
use diesel::migration;
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::*;
use prelude::*;
use glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::common::gtk_box;
use crate::controls::media_controls;
use crate::db::get_connection;
use crate::home::home;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_frame = collection::frame();
        let media_controls = media_controls();
        let title = Rc::new(Label::builder().label("Music player").build());
        let bar = HeaderBar::builder().title_widget(&*title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = Rc::new(MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build());
        let main_box = Rc::new(gtk_box(Vertical));
        main_box.append(&*home());
        main_box.append(&media_controls);
        collection_button.connect_clicked({
            let main_box = main_box.clone();
            let title = title.clone();
            let menu_button = menu_button.clone();
            move |_| {
                update_body(&main_box, &collection_frame, &title, "Collection");
                menu_button.popdown();
            }
        });
        let home_button = Button::builder().icon_name("go-home").tooltip_text("Home").build();
        home_button.connect_clicked({
            let main_box = main_box.clone();
            move |_| { update_body(&main_box, &*home(), &title, "Music player"); }
        });
        bar.pack_start(&home_button);
        bar.pack_end(&*menu_button);
        ApplicationWindow::builder().application(application).child(&*main_box).titlebar(&bar).build().present();
    });
    Ok(application.run())
}

fn update_body(gtk_box: &Rc<Box>, body: &impl IsA<Widget>, title: &Rc<Label>, label: &str) {
    gtk_box.remove(&gtk_box.first_child().unwrap());
    gtk_box.prepend(body);
    title.set_label(label);
}
