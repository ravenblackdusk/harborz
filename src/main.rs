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

trait Removable {
    fn append_collection_remove(&self, path: &Collection);
}

impl Removable for Box {
    fn append_collection_remove(&self, collection: &Collection) {
        let collection_remove = Box::builder().orientation(Horizontal).build();
        let remove_button = Button::builder().label("-").build();
        let id = collection.id;
        remove_button.connect_clicked(clone!(@weak self as borrowed_self, @weak collection_remove => move |_| {
            delete(collections.find(id)).execute(&mut get_connection())
                .expect("should be able to delete collection");
            borrowed_self.remove(&collection_remove);
        }));
        collection_remove.append(&Label::builder().label(&collection.path).build());
        collection_remove.append(&remove_button);
        self.append(&collection_remove);
    }
}

fn main() {
    dotenv().ok();
    std_logger::Config::logfmt().init();
    let application = Application::builder().application_id("eu.agoor.music-player").build();
    application.connect_activate(|application| {
        let collection_box = Box::builder().orientation(Vertical).build();
        for collection in collections.load::<Collection>(&mut get_connection()).expect("should be able to get collections") {
            collection_box.append_collection_remove(&collection);
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
