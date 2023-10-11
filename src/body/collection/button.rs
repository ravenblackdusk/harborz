use std::rc::Rc;
use adw::prelude::*;
use gtk::{Button, MenuButton};
use crate::body::collection::page::COLLECTION;
use crate::common::state::State;

pub(in crate::body) fn create(state: Rc<State>, menu_button: &MenuButton) -> Button {
    let collection_button = Button::builder().label("Collection").build();
    let menu_button = menu_button.clone();
    collection_button.connect_clicked(move |_| {
        state.navigation_view.push_by_tag(COLLECTION);
        menu_button.popdown();
    });
    collection_button
}
