use std::cell::RefCell;
use std::rc::Rc;
use adw::{ApplicationWindow, HeaderBar};
use gtk::{Button, MenuButton, ScrolledWindow};
use crate::body::Body;
use crate::common::window_action::WindowActions;

pub struct State {
    pub window: ApplicationWindow,
    pub header_body: gtk::Box,
    pub header_bar: HeaderBar,
    pub back_button: Button,
    pub window_actions: WindowActions,
    pub menu_button: MenuButton,
    pub scrolled_window: ScrolledWindow,
    pub history: RefCell<Vec<(Rc<Body>, bool)>>,
}
