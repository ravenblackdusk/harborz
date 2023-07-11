mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;
mod home;

use Align::Fill;
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::*;
use gtk::prelude::*;
use gtk::glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::collection::add_collection_box;
use crate::common::gtk_box;
use crate::common::constant::APP_ID;
use crate::controls::media_controls;
use crate::db::get_connection;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(|application| {
        let add_collection_box = add_collection_box();
        let media_controls = media_controls();
        let title = Label::new(Some("Harborz"));
        let bar = HeaderBar::builder().title_widget(&title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build();
        let main_box = Box::builder().spacing(4).orientation(Vertical).valign(Fill).build();
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        home::set_body(&scrolled_window, &media_controls);
        main_box.append(&scrolled_window);
        main_box.append(&media_controls);
        collection_button.connect_clicked({
            let scrolled_window = scrolled_window.clone();
            let title = title.clone();
            let menu_button = menu_button.clone();
            move |_| {
                scrolled_window.set_child(Some(&add_collection_box));
                title.set_label("Collection");
                menu_button.popdown();
            }
        });
        let home_button = Button::builder().icon_name("go-home").tooltip_text("Home").build();
        home_button.connect_clicked({
            let scrolled_window = scrolled_window.clone();
            move |_| {
                home::set_body(&scrolled_window, &media_controls);
                title.set_label("Harborz");
            }
        });
        bar.pack_start(&home_button);
        bar.pack_end(&menu_button);
        let window = ApplicationWindow::builder().application(application).child(&main_box).titlebar(&bar).build();
        window.connect_destroy({
            let application = application.clone();
            move |_| { application.quit(); }
        });
        window.present();
    });
    Ok(application.run())
}
