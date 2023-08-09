use std::cell::RefCell;
use std::rc::Rc;
use adw::{ApplicationWindow, HeaderBar, WindowTitle};
use gtk::{Button, MenuButton, ScrolledWindow};
use crate::body::Body;

pub struct State {
    pub window: ApplicationWindow,
    pub header_body: gtk::Box,
    pub header_bar: HeaderBar,
    pub back_button: Button,
    pub window_title: WindowTitle,
    pub menu_button: MenuButton,
    pub scrolled_window: ScrolledWindow,
    pub history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>,
}
