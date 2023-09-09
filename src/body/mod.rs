use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use adw::prelude::*;
use gtk::{Image, Label, Widget};
use gtk::Orientation::Vertical;
use crate::body::download::albums::albums;
use crate::body::artists::artists;
use crate::body::collection::body::collections;
use crate::body::merge::MergeState;
use crate::body::download::songs::songs_body;
use crate::common::AdjustableScrolledWindow;
use crate::common::state::State;

pub mod collection;
mod merge;
pub mod artists;
mod download;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::bodies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BodyTable {
    pub id: i32,
    pub body_type: BodyType,
    pub scroll_adjustment: Option<f32>,
    pub navigation_type: NavigationType,
    pub params: String,
}

#[derive(Debug, PartialEq, diesel_derive_enum::DbEnum)]
pub enum BodyType {
    Artists,
    Albums,
    Songs,
    Collections,
}

#[derive(Debug, diesel_derive_enum::DbEnum)]
pub enum NavigationType {
    History,
    SongSelected,
}

pub struct Body {
    back_visible: bool,
    title: Arc<String>,
    subtitle: Rc<String>,
    popover_box: gtk::Box,
    pub body_type: BodyType,
    pub params: Vec<Option<Arc<String>>>,
    pub scroll_adjustment: Cell<Option<f32>>,
    pub widget: Box<dyn AsRef<Widget>>,
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").margin_start(10).margin_end(8).build()
}

trait Castable {
    fn first_child(self) -> Option<Widget>;
    fn set_label(self, label: &str) -> Label;
}

impl Castable for Option<Widget> {
    fn first_child(self) -> Option<Widget> {
        self.and_downcast::<gtk::Box>().unwrap().first_child()
    }
    fn set_label(self, label: &str) -> Label {
        let result = self.and_downcast::<Label>().unwrap();
        result.set_label(label);
        result
    }
}

const ARTIST: &'static str = "Artist";
const ALBUM: &'static str = "Album";
const SONG: &'static str = "Song";

fn popover_box(state: Rc<State>, merge_state: Rc<MergeState>) -> gtk::Box {
    let gtk_box = gtk::Box::builder().orientation(Vertical).build();
    gtk_box.append(&collection::button::create(state.clone()));
    gtk_box.append(&merge_state.merge_menu_button);
    gtk_box
}

impl Body {
    pub fn from_body_table(body_type: BodyType, params: Vec<Option<String>>, state: Rc<State>) -> Self {
        let params = params.into_iter().map(|it| { it.map(Arc::new) }).collect();
        match body_type {
            BodyType::Artists => { artists(state) }
            BodyType::Albums => { albums(params, state) }
            BodyType::Songs => { songs_body(params, state) }
            BodyType::Collections => { collections(state) }
        }
    }
    pub fn put_to_history(self, scroll_adjustment: Option<f32>, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) {
        self.scroll_adjustment.set(scroll_adjustment);
        history.borrow_mut().push((Rc::new(self), true));
    }
    pub fn set(self: Rc<Self>, state: Rc<State>) {
        state.back_button.set_visible(self.back_visible);
        state.window_actions.change_window_title.activate(&*self.title);
        state.window_actions.change_window_subtitle.activate(&*self.subtitle);
        state.menu_button.set_visible(if self.popover_box.first_child() == None {
            false
        } else {
            state.menu_button.popover().unwrap().set_child(Some(&self.popover_box));
            true
        });
        state.scrolled_window.set_child(Some((*self.widget).as_ref()));
    }
    pub fn set_with_history(self: Rc<Self>, state: Rc<State>) {
        self.clone().set(state.clone());
        let mut history = state.history.borrow_mut();
        if let Some((body, _)) = history.last() {
            let Body { scroll_adjustment, .. } = body.deref();
            scroll_adjustment.set(state.scrolled_window.get_adjustment());
        }
        history.push((self, false));
    }
}
