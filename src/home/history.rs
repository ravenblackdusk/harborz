use std::cell::RefCell;
use std::rc::Rc;
use adw::glib::GString;
use adw::WindowTitle;
use gtk::{ScrolledWindow, Widget};

pub trait History {
    fn push_window(&self, widow_title: &WindowTitle, scrolled_window: &ScrolledWindow);
}

impl History for Rc<RefCell<Vec<(GString, GString, Box<dyn AsRef<Widget>>)>>> {
    fn push_window(&self, widow_title: &WindowTitle, scrolled_window: &ScrolledWindow) {
        self.borrow_mut().push((widow_title.title(), widow_title.subtitle(),
            Box::new(scrolled_window.child().unwrap())));
    }
}
