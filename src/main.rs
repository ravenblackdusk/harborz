use std::cell::RefCell;
use std::rc::Rc;
use adw::{Application, ApplicationWindow, HeaderBar};
use adw::glib::signal::Inhibit;
use adw::prelude::*;
use diesel::{ExpressionMethods, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Button, Label, MenuButton, Popover, ScrolledWindow, Widget};
use gtk::Align::Fill;
use gtk::glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::collection::add_collection_box;
use crate::common::constant::APP_ID;
use crate::common::gtk_box;
use crate::config::Config;
use crate::controls::media_controls;
use crate::controls::playbin::{PLAYBIN, Playbin};
use crate::db::get_connection;
use crate::schema::config::{current_song_position, maximized, window_height, window_width};
use crate::schema::config::dsl::config as config_table;

mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;
mod home;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(|application| {
        let main_box = gtk::Box::builder().orientation(Vertical).valign(Fill).build();
        let back_button = Button::builder().icon_name("go-previous-symbolic").tooltip_text("Home").build();
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let window = ApplicationWindow::builder().application(application).content(&main_box)
            .default_width(config.window_width).default_height(config.window_height).maximized(config.maximized == 1)
            .build();
        let add_collection_box = add_collection_box(&window);
        let media_controls = media_controls();
        let title = Label::new(Some("Harborz"));
        let bar = HeaderBar::builder().title_widget(&title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build();
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        let history: Rc<RefCell<Vec<Box<dyn AsRef<Widget>>>>> = Rc::new(RefCell::new(Vec::new()));
        back_button.connect_clicked({
            let history = history.clone();
            let scrolled_window = scrolled_window.clone();
            let title = title.clone();
            move |_| {
                if let Some(last_child) = history.borrow_mut().pop() {
                    scrolled_window.set_child(Some((*last_child).as_ref()));
                    title.set_label("Harborz");
                }
            }
        });
        collection_button.connect_clicked({
            let history = history.clone();
            let scrolled_window = scrolled_window.clone();
            let menu_button = menu_button.clone();
            move |_| {
                history.borrow_mut().push(Box::new(scrolled_window.child().unwrap()));
                scrolled_window.set_child(Some(&add_collection_box));
                title.set_label("Collection");
                menu_button.popdown();
            }
        });
        home::set_body(&scrolled_window, history, &media_controls);
        main_box.append(&bar);
        main_box.append(&scrolled_window);
        main_box.append(&media_controls);
        bar.pack_start(&back_button);
        bar.pack_end(&menu_button);
        window.connect_close_request(|window| {
            let (width, height) = window.default_size();
            update(config_table).set((window_width.eq(width), window_height.eq(height),
                maximized.eq(if window.is_maximized() { 1 } else { 0 }),
                current_song_position.eq(PLAYBIN.get_position().unwrap_or(0) as i64)
            )).execute(&mut get_connection()).unwrap();
            Inhibit(false)
        });
        window.present();
    });
    Ok(application.run())
}
