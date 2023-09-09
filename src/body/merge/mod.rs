mod r#impl;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use adw::prelude::*;
use diesel::BoxableExpression;
use diesel::dsl::InnerJoinQuerySource;
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use gtk::Button;
use crate::common::state::State;
use crate::schema::collections::dsl::collections;
use crate::schema::songs::dsl::songs;

pub(super) struct MergeState {
    entity: &'static str,
    state: Rc<State>,
    title: Arc<String>,
    subtitle: Rc<String>,
    merging: Cell<bool>,
    entities_box: gtk::Box,
    selected_for_merge: RefCell<HashSet<gtk::Box>>,
    cancel_button: Button,
    merge_button: Button,
    pub merge_menu_button: Button,
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
