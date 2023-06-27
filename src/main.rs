mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;
mod home;

use std::rc::Rc;
use Align::Fill;
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::*;
use gtk::prelude::*;
use gtk::glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::collection::collection_box;
use crate::common::{box_builder, gtk_box};
use crate::controls::media_controls;
use crate::db::get_connection;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_box = collection_box();
        let media_controls = Rc::new(media_controls());
        let title = Rc::new(Label::new(Some("Music player")));
        let bar = HeaderBar::builder().title_widget(&*title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = Rc::new(MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build());
        let main_box = box_builder().orientation(Vertical).valign(Fill).build();
        let scrolled_window = Rc::new(ScrolledWindow::builder().vexpand(true).build());
        home::set_body(scrolled_window.clone(), media_controls.clone());
        main_box.append(&*scrolled_window);
        main_box.append(&*media_controls);
        collection_button.connect_clicked({
            let scrolled_window = scrolled_window.clone();
            let title = title.clone();
            let menu_button = menu_button.clone();
            move |_| {
                scrolled_window.set_child(Some(&collection_box));
                title.set_label("Collection");
                menu_button.popdown();
            }
        });
        let home_button = Button::builder().icon_name("go-home").tooltip_text("Home").build();
        home_button.connect_clicked({
            let scrolled_window = scrolled_window.clone();
            move |_| {
                home::set_body(scrolled_window.clone(), media_controls.clone());
                title.set_label("Music player");
            }
        });
        bar.pack_start(&home_button);
        bar.pack_end(&*menu_button);
        ApplicationWindow::builder().application(application).child(&main_box).titlebar(&bar).build().present();
    });
    Ok(application.run())
}
