mod choose_file;
mod schema;
mod models;
mod db;

use diesel::{delete, QueryDsl, RunQueryDsl};
use dotenvy::dotenv;
use gtk::*;
use prelude::*;
use traits::{BoxExt, ButtonExt, GridExt};
use Orientation::Vertical;
use glib::clone;
use gtk::Orientation::Horizontal;
use crate::choose_file::choose_file;
use crate::db::get_connection;
use crate::models::Collection;
use crate::schema::collections::dsl::collections;

fn main() {
    dotenv().ok();
    std_logger::Config::logfmt().init();
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_box = Box::builder().orientation(Vertical).build();
        for collection in collections.load::<Collection>(&mut get_connection()).expect("should be able to get collections") {
            let collection_remove = Box::builder().orientation(Horizontal).build();
            let remove_button = Button::builder().label("-").build();
            remove_button.connect_clicked(clone!(@weak collection_box, @weak collection_remove => move |_| {
                delete(collections.find(collection.id)).execute(&mut get_connection())
                    .expect("should be able to delete collection");
                collection_box.remove(&collection_remove);
            }));
            collection_remove.append(&Label::builder().label(collection.path).build());
            collection_remove.append(&remove_button);
            collection_box.append(&collection_remove);
        }
        let browse_button = Button::builder().label("browse").build();
        browse_button.connect_clicked(clone!(@weak collection_box => move |_| { choose_file(&collection_box); }));
        let grid = Grid::builder().build();
        grid.attach(&collection_box, 0, 0, 1, 1);
        grid.attach(&browse_button, 0, 1, 1, 1);
        ApplicationWindow::builder().application(application).title("music player").child(&grid)
            .build().present();
    });
    application.run();
}
