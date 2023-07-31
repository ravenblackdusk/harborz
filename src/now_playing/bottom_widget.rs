use std::cell::RefCell;
use std::rc::Rc;
use gtk::{Button, CssProvider, GestureClick, GestureLongPress, GestureSwipe, Label, ScrolledWindow, style_context_add_provider_for_display, STYLE_PROVIDER_PRIORITY_APPLICATION};
use adw::prelude::*;
use adw::WindowTitle;
use gtk::Orientation::Vertical;
use crate::body::Body;
use crate::common::{BoldLabelBuilder, EllipsizedLabelBuilder};
use crate::now_playing::now_playing::NowPlaying;

pub(in crate::now_playing) trait Playable {
    fn change_state(&self, icon: &str, tooltip: &str);
    fn play(&self);
    fn pause(&self);
}

impl Playable for Button {
    fn change_state(&self, icon: &str, tooltip: &str) {
        self.set_icon_name(icon);
        self.set_tooltip_text(Some(tooltip));
    }
    fn play(&self) {
        self.change_state("media-playback-start", "Play");
    }
    fn pause(&self) {
        self.change_state("media-playback-pause", "Pause");
    }
}

pub(in crate::now_playing) fn create(now_playing: Rc<RefCell<NowPlaying>>,
    song_selected_body: Rc<RefCell<Option<Rc<Body>>>>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
    history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Button)
    -> (gtk::Box, GestureSwipe, GestureClick, Label, Label, Button) {
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
        move |_, _, _| {
            if let Some(body) = song_selected_body.borrow().as_ref() {
                body.clone().set(&window_title, &scrolled_window, history.clone(), &Some(back_button.clone()));
            }
        }
    });
    now_playing.borrow().bottom_image.add_controller(song_selected_body_gesture);
    let image_click = GestureClick::new();
    now_playing.borrow().bottom_image.add_controller(image_click.clone());
    let song_info = gtk::Box::builder().orientation(Vertical).margin_start(4).build();
    now_playing_box.append(&song_info);
    let song_label = Label::builder().margin_ellipsized(4).bold().build();
    song_info.append(&song_label);
    let artist_label = Label::builder().margin_ellipsized(4).build();
    song_info.append(&artist_label);
    song_info.append(&now_playing.borrow().bottom_position);
    let play_pause = Button::builder().width_request(40).build();
    now_playing_and_play_pause.append(&play_pause);
    play_pause.play();
    play_pause.add_css_class("flat");
    (now_playing_and_progress, skip_song_gesture, image_click, song_label, artist_label, play_pause)
}
