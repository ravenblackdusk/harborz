use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;
use gtk::Orientation::Vertical;
use crate::now_playing::now_playing::NowPlaying;

pub(in crate::now_playing) fn create(now_playing: Rc<RefCell<NowPlaying>>) -> gtk::Box {
    let body = gtk::Box::builder().orientation(Vertical).build();
    body.append(&now_playing.borrow().body_image);
    let time = gtk::Box::builder().margin_start(8).margin_end(8).build();
    body.append(&time);
    time.append(&now_playing.borrow().body_position);
    time.append(&now_playing.borrow().scale);
    time.append(&now_playing.borrow().body_duration);
    body.append(&now_playing.borrow().body_play_pause);
    body
}
