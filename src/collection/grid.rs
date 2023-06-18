use diesel::{delete, QueryDsl, RunQueryDsl};
use gtk::{glib, prelude, Button, Grid, Label};
use glib::clone;
use prelude::*;
use crate::db::get_connection;
use crate::collection::Collection;
use crate::schema::collections::dsl::collections;

pub(in crate::collection) trait CollectionGrid {
    fn add(&self, collection: &Collection, row: i32);

    fn new() -> Self;
}

impl CollectionGrid for Grid {
    fn add(&self, collection: &Collection, row: i32) {
        let remove_button = Button::builder().icon_name("list-remove").build();
        let id = collection.id;
        remove_button.connect_clicked(clone!(@weak self as borrowed_self => move |_| {
            delete(collections.find(id)).execute(&mut get_connection())
                .expect("should be able to delete collection");
            borrowed_self.remove_row(row);
        }));
        self.attach(&Label::builder().label(&collection.path).build(), 0, row, 1, 1);
        self.attach(&remove_button, 1, row, 1, 1);
    }

    fn new() -> Self {
        let collection_grid = Grid::builder().row_spacing(4).column_spacing(4).build();
        for (row, collection) in collections.load::<Collection>(&mut get_connection())
            .expect("should be able to get collections").into_iter().enumerate() {
            collection_grid.add(&collection, row as i32);
        }
        collection_grid
    }
}
