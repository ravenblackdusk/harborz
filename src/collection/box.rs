use std::ops::Add;
use diesel::{Connection, delete, ExpressionMethods, QueryDsl, RunQueryDsl, update};
use gtk::{Button, Label, prelude};
use gtk::Orientation::{Horizontal, Vertical};
use prelude::*;
use crate::collection::model::Collection;
use crate::common::{EllipsizedLabelBuilder, gtk_box};
use crate::common::util::PathString;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::row;

pub(in crate::collection) trait CollectionBox {
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
        inner_box.append(&Label::builder().ellipsized().label(path.to_path().file_name().unwrap().to_str().unwrap())
            .build());
        inner_box.append(&remove_button);
        self.append(&inner_box);
        remove_button.connect_clicked({
            let this = self.clone();
            move |_| {
                get_connection().transaction(|connection| {
                    let db_collection = delete(collections.find(id)).get_result::<Collection>(connection)?;
                    update(collections.filter(row.gt(db_collection.row))).set(row.eq(row.add(-1))).execute(connection)
                }).unwrap();
                this.remove(&inner_box);
            }
        });
    }
}
