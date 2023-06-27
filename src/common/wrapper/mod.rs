mod imp;

use gtk::{glib, Widget};
use gtk::glib::{Cast, IsA, Object};

pub const SONG_SELECTED: &'static str = "song-selected";

glib::wrapper! {
    pub struct Wrapper(ObjectSubclass<imp::Wrapper>) @extends gtk::Widget, @implements gtk::Accessible;
}

impl Wrapper {
    pub fn new(child: &impl IsA<Widget>) -> Self {
        Object::builder().property("child", child.clone().upcast()).build()
    }
}
