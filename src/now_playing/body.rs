use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;
use gtk::{Button, GestureSwipe, Image};
use gtk::Orientation::Vertical;
use crate::common::StyledWidget;
use crate::now_playing::now_playing::NowPlaying;
use crate::now_playing::playbin::{PLAYBIN, Playbin};

pub(super) fn create(now_playing: Rc<RefCell<NowPlaying>>) -> (gtk::Box, GestureSwipe) {
    let body = gtk::Box::builder().orientation(Vertical).margin_bottom(48).build();
    let image_and_song_info = gtk::Box::builder().orientation(Vertical).build();
    body.append(&image_and_song_info);
    image_and_song_info.append(&now_playing.borrow().body_image);
    let song_info = gtk::Box::builder().orientation(Vertical).spacing(4).margin_start(8).margin_end(8).margin_bottom(4)
        .build();
    image_and_song_info.append(&song_info);
    let skip_song_gesture = GestureSwipe::new();
    image_and_song_info.add_controller(skip_song_gesture.clone());
    song_info.append(&now_playing.borrow().body_song);
    song_info.append(&now_playing.borrow().body_artist);
    let time_and_controls = gtk::Box::builder().orientation(Vertical).margin_start(8).margin_end(8).build();
    body.append(&time_and_controls);
    time_and_controls.append(&now_playing.borrow().scale);
    let time = gtk::Box::builder().margin_start(11).margin_end(11).margin_bottom(4).build();
    time_and_controls.append(&time);
    time.append(&now_playing.borrow().body_position);
    time.append(&now_playing.borrow().body_duration);
    let controls = gtk::Box::builder().build();
    time_and_controls.append(&controls);
    let skip_backward = Button::builder().hexpand(true).tooltip_text("Previous")
        .child(&Image::builder().icon_name("media-skip-backward").pixel_size(28).build()).build().flat();
    skip_backward.connect_clicked(|_| { PLAYBIN.go_delta_song(-1, true); });
    let skip_forward = Button::builder().hexpand(true).tooltip_text("Next")
        .child(&Image::builder().icon_name("media-skip-forward").pixel_size(28).build()).build().flat();
    skip_forward.connect_clicked(|_| { PLAYBIN.go_delta_song(1, true); });
    controls.append(&skip_backward);
    controls.append(&now_playing.borrow().body_play_pause);
    controls.append(&skip_forward);
    (body, skip_song_gesture)
}
