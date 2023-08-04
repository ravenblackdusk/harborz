use std::cell::RefCell;
use std::rc::Rc;
use adw::prelude::*;
use adw::WindowTitle;
use gtk::{Button, CssProvider, EventSequenceState, GestureClick, GestureLongPress, GestureSwipe, ScrolledWindow, style_context_add_provider_for_display, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::Orientation::Vertical;
use crate::body::Body;
use crate::now_playing::now_playing::NowPlaying;

pub(in crate::now_playing) fn create(now_playing: Rc<RefCell<NowPlaying>>,
    song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
    history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Button)
    -> (gtk::Box, GestureSwipe, GestureClick) {
    let now_playing_and_progress = gtk::Box::builder().orientation(Vertical).name("accent-bg").build();
    let css_provider = CssProvider::new();
    css_provider.load_from_data("#accent-bg { background-color: @accent_bg_color; } \
    #accent-progress progress { background-color: @accent_fg_color; }");
    style_context_add_provider_for_display(&now_playing_and_progress.display(), &css_provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION);
    now_playing.borrow().progress_bar.add_css_class("osd");
    now_playing_and_progress.append(&now_playing.borrow().progress_bar);
    let now_playing_and_play_pause = gtk::Box::builder().margin_start(8).margin_end(8).margin_top(8).margin_bottom(8)
        .build();
    now_playing_and_progress.append(&now_playing_and_play_pause);
    let now_playing_box = gtk::Box::builder().build();
    now_playing_and_play_pause.append(&now_playing_box);
    let skip_song_gesture = GestureSwipe::new();
    now_playing_box.add_controller(skip_song_gesture.clone());
    now_playing_box.append(&now_playing.borrow().bottom_image);
    let song_selected_body_gesture = GestureLongPress::new();
    song_selected_body_gesture.connect_pressed({
        let song_selected_body = song_selected_body.clone();
        let window_title = window_title.clone();
        let scrolled_window = scrolled_window.clone();
        let history = history.clone();
        let back_button = back_button.clone();
        move |gesture, _, _| {
            if let Some(body) = song_selected_body.borrow().as_ref() {
                let song_selected_body_realized = if let Some((last, _)) = history.borrow().last() {
                    Rc::ptr_eq(last, body)
                } else {
                    return;
                };
                if !song_selected_body_realized {
                    gesture.set_state(EventSequenceState::Claimed);
                    body.clone().set(&window_title, &scrolled_window, history.clone(), &Some(back_button.clone()));
                }
            }
        }
    });
    now_playing.borrow().bottom_image.connect_realize(move |bottom_image| {
        bottom_image.add_controller(song_selected_body_gesture.clone());
    });
    let image_click = GestureClick::new();
    now_playing.borrow().bottom_image.add_controller(image_click.clone());
    let song_info = gtk::Box::builder().orientation(Vertical).margin_start(4).build();
    now_playing_box.append(&song_info);
    song_info.append(&now_playing.borrow().song_label);
    song_info.append(&now_playing.borrow().artist_label);
    song_info.append(&now_playing.borrow().bottom_position);
    now_playing_and_play_pause.append(&now_playing.borrow().bottom_play_pause);
    (now_playing_and_progress, skip_song_gesture, image_click)
}
