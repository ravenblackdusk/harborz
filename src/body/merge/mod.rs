use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use adw::prelude::*;
use diesel::BoxableExpression;
use diesel::dsl::InnerJoinQuerySource;
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use gtk::{Button, MenuButton};
use crate::body::{action_name, START_MERGE};
use crate::schema::collections::dsl::collections;
use crate::schema::songs::dsl::songs;

mod r#impl;

pub(super) struct MergeState {
    entity: &'static str,
    title: Arc<String>,
    subtitle: Rc<String>,
    merging: Cell<bool>,
    entities_box: gtk::Box,
    selected_for_merge: RefCell<HashSet<gtk::Box>>,
    merge_button: Button,
}

pub(super) const KEY: &'static str = "key";

type Query = Box<dyn BoxableExpression<InnerJoinQuerySource<songs, collections>, Sqlite, SqlType=Bool>>;

trait MergeButton {
    fn disable(&self);
}

impl MergeButton for Button {
    fn disable(&self) {
        self.set_sensitive(false);
        self.set_tooltip_text(Some("Select 2 or more"));
    }
}

pub fn add_menu_merge_button(entity: &str, menu_button: &MenuButton, popover_box: &gtk::Box) -> Rc<String> {
    let heading = format!("Merge {entity}s");
    let merge_menu_button = Button::builder().label(&heading).build();
    let menu_button = menu_button.clone();
    merge_menu_button.connect_clicked(move |merge_menu_button| {
        merge_menu_button.activate_action(&action_name(START_MERGE), None).unwrap();
        menu_button.popdown();
    });
    popover_box.append(&merge_menu_button);
    Rc::new(heading)
}
