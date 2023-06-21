use std::ops::Add;
use std::rc::Rc;
use diesel::{Connection, delete, ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::result::Error;
use gtk::{prelude, Button, Grid, Label};
use prelude::*;
use crate::collection::model::Collection;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::row;

pub(in crate::collection) trait CollectionGrid {
    fn add(self: Rc<Self>, collection: &Collection);

    fn new() -> Rc<Self>;
}

impl CollectionGrid for Grid {
    fn add(self: Rc<Self>, collection: &Collection) {
        let remove_button = Button::builder().icon_name("list-remove").build();
        self.attach(&Label::builder().label(&collection.path).hexpand(true).build(), 0, collection.row, 1, 1);
        self.attach(&remove_button, 1, collection.row, 1, 1);
        let id = collection.id;
        remove_button.connect_clicked(move |_| {
            let db_collection = get_connection().transaction::<_, Error, _>(|connection| {
                let db_collection = delete(collections.find(id)).get_result::<Collection>(connection)?;
                update(collections.filter(row.gt(db_collection.row))).set(row.eq(row.add(-1))).execute(connection)?;
                Ok(db_collection)
            }).expect("should be able to delete collection row");
            self.remove_row(db_collection.row);
        });
    }

    fn new() -> Rc<Self> {
        let collection_grid = Rc::new(Grid::builder().row_spacing(4).column_spacing(4).build());
        for collection in collections.load::<Collection>(&mut get_connection()).expect("should be able to get collections") {
            collection_grid.clone().add(&collection);
        }
        collection_grid
    }
}
