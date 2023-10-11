use std::rc::Rc;
use adw::{HeaderBar, NavigationPage, WindowTitle};
use adw::prelude::*;
use gtk::Orientation::Vertical;
use gtk::ScrolledWindow;
use crate::body::{BodyType, create_navigation_page};
use crate::body::collection::add_collection_box;
use crate::common::state::State;

pub const COLLECTION: &'static str = "Collection";

pub fn collection_page(state: Rc<State>) -> NavigationPage {
    let child = gtk::Box::builder().orientation(Vertical).build();
    child.append(&HeaderBar::builder().title_widget(&WindowTitle::builder().title("Harborz").subtitle("Collection")
        .build()).build());
    child.append(&ScrolledWindow::builder().vexpand(true).child(&add_collection_box(state)).build());
    create_navigation_page(&child, COLLECTION, Vec::new(), BodyType::Collections)
}
