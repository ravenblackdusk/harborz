use std::rc::Rc;
use diesel::{delete, QueryDsl, RunQueryDsl};
use gtk::{prelude, Button, Grid, Label};
use prelude::*;
use crate::db::get_connection;
use crate::collection::Collection;
use crate::schema::collections::dsl::collections;

pub(in crate::collection) trait CollectionGrid {
    fn add(self: Rc<Self>, collection: &Collection, row: i32);

    fn new() -> Rc<Self>;
}

impl CollectionGrid for Grid {
    fn add(self: Rc<Self>, collection: &Collection, row: i32) {
        let remove_button = Button::builder().icon_name("list-remove").build();
        let id = collection.id;
        self.attach(&Label::builder().label(&collection.path).build(), 0, row, 1, 1);
        self.attach(&remove_button, 1, row, 1, 1);
        remove_button.connect_clicked(move |_| {
            delete(collections.find(id)).execute(&mut get_connection())
                .expect("should be able to delete collection");
            self.clone().remove_row(row);
        });
    }

    fn new() -> Rc<Self> {
        let collection_grid = Rc::new(Grid::builder().row_spacing(4).column_spacing(4).build());
        for (row, collection) in collections.load::<Collection>(&mut get_connection())
            .expect("should be able to get collections").into_iter().enumerate() {
            collection_grid.clone().add(&collection, row as i32);
        }
        collection_grid
    }
}
