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
use gtk::*;
use prelude::*;
use glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::common::gtk_box;
use crate::controls::media_controls;
use crate::db::get_connection;

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
        let window = Rc::new(ApplicationWindow::builder().application(application).child(&media_controls).titlebar(&bar).build());
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = Rc::new(MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build());
        collection_button.connect_clicked({
            let window = window.clone();
            let title = title.clone();
            let menu_button = menu_button.clone();
            move |_| {
                update_window(&window, &collection_frame, &title, "Collection");
                menu_button.popdown();
            }
        });
        let home = Button::builder().icon_name("go-home").tooltip_text("Home").build();
        home.connect_clicked({
            let window = window.clone();
            move |_| { update_window(&window, &media_controls, &title, "Music player"); }
        });
        bar.pack_start(&home);
        bar.pack_end(&*menu_button);
        window.present();
    });
    Ok(application.run())
}

fn update_window(window: &Rc<ApplicationWindow>, widget: &impl IsA<Widget>, title: &Rc<Label>, label: &str) {
    window.set_child(Some(widget));
    title.set_label(label);
}
