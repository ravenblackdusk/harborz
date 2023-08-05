use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;
use gtk::{Button, Image};
use gtk::Orientation::Vertical;
use crate::common::FlatButton;
use crate::now_playing::now_playing::NowPlaying;
use crate::now_playing::playbin::{PLAYBIN, Playbin};

pub(in crate::now_playing) fn create(now_playing: Rc<RefCell<NowPlaying>>) -> gtk::Box {
    let body = gtk::Box::builder().orientation(Vertical).build();
    body.append(&now_playing.borrow().body_image);
    let info = gtk::Box::builder().orientation(Vertical).margin_start(8).margin_end(8).build();
    body.append(&info);
    info.append(&now_playing.borrow().body_song);
    info.append(&now_playing.borrow().body_artist);
    info.append(&now_playing.borrow().scale);
    let time = gtk::Box::builder().build();
    info.append(&time);
    time.append(&now_playing.borrow().body_position);
    time.append(&now_playing.borrow().body_duration);
    let controls = gtk::Box::builder().build();
    info.append(&controls);
    let skip_backward = Button::builder().hexpand(true).tooltip_text("Previous")
        .child(&Image::builder().icon_name("media-skip-backward").pixel_size(40).build()).build().flat();
    skip_backward.connect_clicked(|_| { PLAYBIN.go_delta_song(-1, true); });
    let skip_forward = Button::builder().hexpand(true).tooltip_text("Next")
        .child(&Image::builder().icon_name("media-skip-forward").pixel_size(40).build()).build().flat();
    skip_forward.connect_clicked(|_| { PLAYBIN.go_delta_song(1, true); });
    controls.append(&skip_backward);
    controls.append(&now_playing.borrow().body_play_pause);
    controls.append(&skip_forward);
    body
}
