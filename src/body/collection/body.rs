use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use gtk::Orientation::Vertical;
use crate::body::{Body, BodyType};
use crate::body::collection::add_collection_box;
use crate::common::state::State;

pub fn collections(state: Rc<State>) -> Body {
    Body {
        back_visible: true,
        title: Arc::new(String::from("Harborz")),
        subtitle: Rc::new(String::from("Collection")),
        popover_box: gtk::Box::builder().orientation(Vertical).build(),
        body_type: BodyType::Collections,
        params: Vec::new(),
        scroll_adjustment: Cell::new(None),
        widget: Box::new(add_collection_box(state)),
    }
}
