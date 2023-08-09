use std::rc::Rc;
use adw::prelude::*;
use gtk::Button;
use crate::body::{Body, BodyType};
use crate::common::state::State;

pub(in crate::body) fn create(state: Rc<State>) -> Button {
    let collection_button = Button::builder().label("Collection").build();
    collection_button.connect_clicked({
        move |_| {
            if state.history.borrow().last().unwrap().0.body_type != BodyType::Collections {
                Rc::new(Body::collections(state.clone())).set_with_history(state.clone());
            }
            state.menu_button.popdown();
        }
    });
    collection_button
}
