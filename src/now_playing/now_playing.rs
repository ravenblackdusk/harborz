use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use adw::prelude::*;
use gstreamer::ClockTime;
use gstreamer::prelude::ElementExtManual;
use gtk::{Button, Image, Label, ProgressBar, Scale};
use gtk::Align::{End, Start};
use crate::common::{ImagePathBuf, SONG_ICON, StyledLabelBuilder, StyledWidget};
use crate::common::state::State;
use crate::common::util::{format, format_pad};
use crate::now_playing::playbin::PLAYBIN;

pub(super) struct PlayPauseInfo {
    icon_name: &'static str,
    tooltip: &'static str,
}

const PLAY: PlayPauseInfo = PlayPauseInfo {
    icon_name: "media-playback-start",
    tooltip: "Play",
};
const PAUSE: PlayPauseInfo = PlayPauseInfo {
    icon_name: "media-playback-pause",
    tooltip: "Pause",
};

pub(super) trait Playable {
    fn change_state(&self, play_pause_info: PlayPauseInfo);
    fn play(&self);
    fn pause(&self);
}

impl Playable for Button {
    fn change_state(&self, play_pause_info: PlayPauseInfo) {
        self.child().and_downcast::<Image>().unwrap().set_icon_name(Some(play_pause_info.icon_name));
        self.set_tooltip_text(Some(play_pause_info.tooltip));
    }
    fn play(&self) {
        self.change_state(PLAY);
    }
    fn pause(&self) {
        self.change_state(PAUSE);
    }
}

pub struct NowPlaying {
    pub cover: Option<PathBuf>,
    pub bottom_image: Image,
    pub body_image: Image,
    pub position: u64,
    pub duration: u64,
    pub progress_bar: ProgressBar,
    pub scale: Scale,
    pub bottom_position: Label,
    pub body_position: Label,
    pub bottom_duration: Label,
    pub body_duration: Label,
    pub bottom_play_pause: Button,
    pub body_play_pause: Button,
    pub song: String,
    pub artist: String,
    pub bottom_song: Label,
    pub body_song: Label,
    pub bottom_artist: Label,
    pub body_artist: Label,
}

impl NowPlaying {
    fn flat_play(button: Button) -> Button {
        button.play();
        button.flat()
    }
    pub(super) fn new() -> Self {
        let scale = Scale::builder().hexpand(true).name("small-slider").build();
        scale.set_range(0.0, 1.0);
        NowPlaying {
            cover: None,
            bottom_image: Image::builder().pixel_size(56).build(),
            body_image: Image::builder().pixel_size(360).vexpand(true).build(),
            position: 0,
            duration: 0,
            progress_bar: ProgressBar::builder().build().osd(),
            scale,
            bottom_position: Label::builder().label(&format(0)).subscript().build().numeric(),
            body_position: Label::builder().label(&format(0)).hexpand(true).halign(Start).subscript().build().numeric(),
            bottom_duration: Label::builder().label(&format(0)).subscript().build().numeric(),
            body_duration: Label::builder().label(&format(0)).hexpand(true).halign(End).subscript().build().numeric(),
            bottom_play_pause: Self::flat_play(Button::builder()
                .child(&Image::builder().pixel_size(32).build()).build()),
            body_play_pause: Self::flat_play(Button::builder().hexpand(true)
                .child(&Image::builder().pixel_size(40).build()).build()),
            song: String::from(""),
            artist: String::from(""),
            bottom_song: Label::builder().ellipsized().bold().build(),
            body_song: Label::builder().ellipsized().build().with_css_class("title-3"),
            bottom_artist: Label::builder().ellipsized().build(),
            body_artist: Label::builder().ellipsized().build(),
        }
    }
    pub(super) fn click_play_pause(&self) {
        if self.bottom_play_pause.is_realized() { &self.bottom_play_pause } else { &self.body_play_pause }
            .emit_clicked();
    }
    fn update_song_info(&self, state: Rc<State>, other: bool) {
        let (song, artist) = if self.bottom_song.is_realized() != other {
            (&self.bottom_song, &self.bottom_artist)
        } else {
            state.window_actions.change_window_title.activate(&self.song);
            state.window_actions.change_window_subtitle.activate(&self.artist);
            (&self.body_song, &self.body_artist)
        };
        song.set_label(&self.song);
        artist.set_label(&self.artist);
    }
    fn update_image(&self, other: bool) {
        if let Some(cover) = &self.cover {
            if self.body_image.is_realized() != other { &self.body_image } else { &self.bottom_image }
                .set_or_default(cover, SONG_ICON);
        }
    }
    fn update_position(&self, other: bool) {
        if self.bottom_position.is_realized() != other {
            if self.duration != 0 { self.progress_bar.set_fraction(self.position as f64 / self.duration as f64); }
            &self.bottom_position
        } else {
            self.scale.set_value(self.position as f64);
            &self.body_position
        }.set_label(&format_pad(self.position, {
            let minutes = Duration::from_nanos(self.duration).as_secs() / 60;
            if minutes == 0 { 1 } else { (minutes.ilog10() + 1) as usize }
        }));
    }
    fn update_duration_and_position(&self, other: bool) {
        if self.bottom_duration.is_realized() != other {
            &self.bottom_duration
        } else {
            self.scale.set_range(0.0, self.duration as f64);
            &self.body_duration
        }.set_label(&format(self.duration));
        self.update_position(other);
    }
    pub fn set_song_info(&mut self, state: Rc<State>, song: &str, artist: &str) {
        self.song = String::from(song);
        self.artist = String::from(artist);
        self.update_song_info(state, false);
    }
    pub fn set_duration(&mut self) {
        self.duration = PLAYBIN.query_duration().map(ClockTime::nseconds).unwrap_or(0);
        self.update_duration_and_position(false);
    }
    pub fn update_other(&self, state: Rc<State>, icon_name: &str, body: &gtk::Box) {
        self.update_image(true);
        self.update_duration_and_position(true);
        let (current_play_pause, other_play_pause) = if self.bottom_play_pause.is_realized() {
            (&self.bottom_play_pause, &self.body_play_pause)
        } else {
            (&self.body_play_pause, &self.bottom_play_pause)
        };
        if let Some(tooltip) = current_play_pause.tooltip_text() {
            other_play_pause.change_state(if tooltip.as_str() == "Play" { PLAY } else { PAUSE });
        }
        self.update_song_info(state.clone(), true);
        state.back_button.set_visible(true);
        state.back_button.set_icon_name(icon_name);
        state.header_body.remove(&state.header_body.last_child().unwrap());
        state.header_body.append(body);
    }
    pub fn realize_body(&self, state: Rc<State>, body: &gtk::Box) {
        self.update_other(state, "go-down", body);
    }
    pub fn set_album_image(&mut self, cover: PathBuf) -> Option<String> {
        let result = if cover.exists() { cover.to_str().map(|it| { format!("file:{it}") }) } else { None };
        self.cover = Some(cover);
        self.update_image(false);
        result
    }
    pub fn set_position(&mut self, position: u64) {
        self.position = position;
        self.update_position(false);
    }
}
