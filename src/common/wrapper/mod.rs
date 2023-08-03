use adw::glib::{self, Cast, IsA, Object};
use gtk::Widget;

mod imp;

pub const SONG_SELECTED: &'static str = "song-selected";
pub const STREAM_STARTED: &'static str = "stream-started";

glib::wrapper! {
    pub struct Wrapper(ObjectSubclass<imp::Wrapper>) @extends gtk::Widget, @implements gtk::Accessible;
}

impl Wrapper {
    pub fn new(child: &impl IsA<Widget>) -> Self {
        Object::builder().property("child", child.clone().upcast()).build()
    }
}
