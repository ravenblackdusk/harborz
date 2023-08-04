use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::{BoxExt, RangeExt};
use gtk::Label;
use gtk::Orientation::Vertical;
use crate::common::util::format;
use crate::now_playing::now_playing::NowPlaying;

pub(in crate::now_playing) fn create(now_playing: Rc<RefCell<NowPlaying>>) -> (gtk::Box, Label) {
    let body = gtk::Box::builder().orientation(Vertical).build();
    body.append(&now_playing.borrow().body_image);
    let time = gtk::Box::builder().build();
    body.append(&time);
    time.append(&now_playing.borrow().body_position);
    now_playing.borrow().scale.set_range(0.0, 1.0);
    time.append(&now_playing.borrow().scale);
    let duration_label = Label::new(Some(&format(0)));
    time.append(&duration_label);
    body.append(&now_playing.borrow().body_play_pause);
    (body, duration_label)
}
