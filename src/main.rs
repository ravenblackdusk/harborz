mod schema;
mod db;
mod collection;
mod controls;
mod common;
mod config;
mod home;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use Align::Fill;
use diesel::{migration, QueryDsl, RunQueryDsl};
use migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::*;
use prelude::*;
use glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::collection::collection_box;
use crate::common::{box_builder, gtk_box};
use crate::controls::media_controls;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::dsl::config as config_table;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_box = collection_box();
        let path_buf = songs.inner_join(collections).inner_join(config_table).select((path, song_path))
            .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, song_path2)| {
            Path::new(collection_path.as_str()).join(Path::new(song_path2.as_str()))
        }).unwrap_or(PathBuf::from(""));
        let media_file = Rc::new(MediaFile::for_filename(path_buf));
        let media_controls = media_controls(media_file.clone());
        let title = Rc::new(Label::builder().label("Music player").build());
        let bar = HeaderBar::builder().title_widget(&*title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = Rc::new(MenuButton::builder().icon_name("open-menu-symbolic")
            .tooltip_text("Menu").popover(&Popover::builder().child(&menu).build()).build());
        let main_box = box_builder().orientation(Vertical).valign(Fill).build();
        let scrolled_window = Rc::new(ScrolledWindow::builder().vexpand(true).build());
        home::set_body(scrolled_window.clone(), media_file.clone());
        main_box.append(&*scrolled_window);
        main_box.append(&media_controls);
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
                home::set_body(scrolled_window.clone(), media_file.clone());
                title.set_label("Music player");
            }
        });
        bar.pack_start(&home_button);
        bar.pack_end(&*menu_button);
        ApplicationWindow::builder().application(application).child(&main_box).titlebar(&bar).build().present();
    });
    Ok(application.run())
}
