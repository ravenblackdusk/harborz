use std::path::PathBuf;
use std::time::Duration;
use adw::prelude::*;
use adw::WindowTitle;
use gstreamer::ClockTime;
use gstreamer::prelude::ElementExtManual;
use gtk::{Button, Image, Label, ProgressBar, Scale};
use crate::common::{BoldLabelBuilder, EllipsizedLabelBuilder, ImagePathBuf, MonospaceLabel, SONG};
use crate::common::util::{format, format_pad};
use crate::now_playing::playbin::PLAYBIN;

pub(in crate::now_playing) struct PlayPauseInfo {
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

pub(in crate::now_playing) trait Playable {
    fn change_state(&self, play_pause_info: PlayPauseInfo);
    fn play(&self);
    fn pause(&self);
}

impl Playable for Button {
    fn change_state(&self, play_pause_info: PlayPauseInfo) {
        self.set_icon_name(play_pause_info.icon_name);
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
    pub song_label: Label,
    pub artist_label: Label,
}

impl NowPlaying {
    fn flat_play(button: Button) -> Button {
        button.play();
        button.add_css_class("flat");
        button
    }
    pub(in crate::now_playing) fn new() -> Self {
        let scale = Scale::builder().hexpand(true).build();
        scale.set_range(0.0, 1.0);
        NowPlaying {
            cover: None,
            bottom_image: Image::builder().pixel_size(56).build(),
            body_image: Image::builder().pixel_size(360).vexpand(true).build(),
            position: 0,
            duration: 0,
            progress_bar: ProgressBar::builder().name("accent-progress").build(),
            scale,
            bottom_position: Label::builder().label(&format(0)).build().monospace(),
            body_position: Label::builder().label(&format(0)).build().monospace(),
            bottom_duration: Label::new(Some(&format(0))).monospace(),
            body_duration: Label::new(Some(&format(0))).monospace(),
            bottom_play_pause: Self::flat_play(Button::builder().width_request(40).build()),
            body_play_pause: Self::flat_play(Button::builder().build()),
            song: String::from(""),
            artist: String::from(""),
            song_label: Label::builder().ellipsized().bold().build(),
            artist_label: Label::builder().ellipsized().build(),
        }
    }
    pub(in crate::now_playing) fn click_play_pause(&self) {
        if self.bottom_play_pause.is_realized() { &self.bottom_play_pause } else { &self.body_play_pause }
            .emit_clicked();
    }
    fn update_song_info(&self, other: bool, window_title: &WindowTitle) {
        if self.song_label.is_realized() != other {
            self.song_label.set_label(&self.song);
            self.artist_label.set_label(&self.artist);
        } else {
            window_title.set_title(&self.song);
            window_title.set_subtitle(&self.artist);
        }
    }
    fn update_image(&self, other: bool) {
        if let Some(cover) = &self.cover {
            if self.body_image.is_realized() != other { &self.body_image } else { &self.bottom_image }
                .set_cover(cover, SONG);
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
    fn update_duration(&self, other: bool) {
        if self.bottom_duration.is_realized() != other {
            &self.bottom_duration
        } else {
            self.scale.set_range(0.0, self.duration as f64);
            &self.body_duration
        }.set_label(&format(self.duration));
    }
    pub fn set_song_info(&mut self, song: &str, artist: &str, window_title: &WindowTitle) {
        self.song = String::from(song);
        self.artist = String::from(artist);
        self.update_song_info(false, window_title);
    }
    pub fn set_duration(&mut self) {
        self.duration = PLAYBIN.query_duration().map(ClockTime::nseconds).unwrap_or(0);
        self.update_duration(false);
    }
    pub fn update_other(&self, window_title: &WindowTitle, back_button: &Button, icon_name: &str,
        header_body: &gtk::Box, body: &gtk::Box) {
        self.update_image(true);
        self.update_position(true);
        self.update_duration(true);
        let (current_play_pause, other_play_pause) = if self.bottom_play_pause.is_realized() {
            (&self.bottom_play_pause, &self.body_play_pause)
        } else {
            (&self.body_play_pause, &self.bottom_play_pause)
        };
        if let Some(tooltip) = current_play_pause.tooltip_text() {
            other_play_pause.change_state(if tooltip.as_str() == "Play" { PLAY } else { PAUSE });
        }
        self.update_song_info(true, window_title);
        back_button.set_visible(true);
        back_button.set_icon_name(icon_name);
        header_body.remove(&header_body.last_child().unwrap());
        header_body.append(body);
    }
    pub fn set_album_image(&mut self, cover: PathBuf) -> Option<String> {
        let result = if cover.exists() { cover.to_str().map(|it| { format!("file:{}", it) }) } else { None };
        self.cover = Some(cover);
        self.update_image(false);
        result
    }
    pub fn set_position(&mut self, position: u64) {
        self.position = position;
        self.update_position(false);
    }
}
