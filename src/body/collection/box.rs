use diesel::{delete, QueryDsl, RunQueryDsl};
use gtk::{Button, Label, prelude};
use gtk::Orientation::{Horizontal, Vertical};
use prelude::*;
use crate::body::collection::model::Collection;
use crate::common::{EllipsizedLabelBuilder, gtk_box};
use crate::common::util::PathString;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;

pub(in crate::body::collection) trait CollectionBox {
    fn new() -> Self;
    fn add(&self, id: i32, path: &String);
}

impl CollectionBox for gtk::Box {
    fn new() -> Self {
        let gtk_box = gtk_box(Vertical);
        for collection in collections.load::<Collection>(&mut get_connection()).unwrap() {
            gtk_box.clone().add(collection.id, &collection.path);
        }
        gtk_box
    }
    fn add(&self, id: i32, path: &String) {
        let remove_button = Button::builder().icon_name("list-remove").build();
        let inner_box = gtk_box(Horizontal);
        inner_box.append(&Label::builder().label(path.to_path().file_name().unwrap().to_str().unwrap())
            .margin_ellipsized(4).build());
        inner_box.append(&remove_button);
        self.append(&inner_box);
        remove_button.connect_clicked({
            let this = self.clone();
            move |_| {
                delete(collections.find(id)).execute(&mut get_connection()).unwrap();
                this.remove(&inner_box);
            }
        });
    }
}
