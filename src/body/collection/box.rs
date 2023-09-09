use std::cell::RefCell;
use std::rc::Rc;
use diesel::{Connection, delete, QueryDsl, RunQueryDsl};
use gtk::{Button, Label, prelude};
use gtk::Orientation::{Horizontal, Vertical};
use prelude::*;
use crate::body::Body;
use crate::body::collection::model::Collection;
use crate::common::{StyledLabelBuilder, gtk_box};
use crate::common::constant::DESTRUCTIVE_ACTION;
use crate::common::util::PathString;
use crate::db::get_connection;
use crate::schema::bodies::dsl::bodies;
use crate::schema::collections::dsl::collections;

pub(super) trait CollectionBox {
    fn new(history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) -> Self;
    fn add(&self, id: i32, path: &String, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>);
}

impl CollectionBox for gtk::Box {
    fn new(history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) -> Self {
        let gtk_box = gtk_box(Vertical);
        for collection in collections.load::<Collection>(&mut get_connection()).unwrap() {
            gtk_box.clone().add(collection.id, &collection.path, history.clone());
        }
        gtk_box
    }
    fn add(&self, id: i32, path: &String, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) {
        let remove_button = Button::builder().icon_name("list-remove").build();
        remove_button.add_css_class(DESTRUCTIVE_ACTION);
        let inner_box = gtk_box(Horizontal);
        inner_box.append(&Label::builder().label(path.to_path().file_name().unwrap().to_str().unwrap())
            .margin_ellipsized(4).build());
        inner_box.append(&remove_button);
        self.append(&inner_box);
        remove_button.connect_clicked({
            let history = history.clone();
            let this = self.clone();
            move |_| {
                get_connection().transaction(|connection| {
                    delete(collections.find(id)).execute(connection)?;
                    delete(bodies).execute(connection)
                }).unwrap();
                history.borrow_mut().clear();
                this.remove(&inner_box);
            }
        });
    }
}
