use std::path::PathBuf;
use gtk::{Button, Image, Label, ProgressBar, Scale};
use adw::prelude::*;
use gstreamer::ClockTime;
use gstreamer::prelude::ElementExtManual;
use crate::common::ImagePathBuf;
use crate::common::util::format;
use crate::now_playing::playbin::PLAYBIN;

pub struct NowPlaying {
    pub cover: Option<PathBuf>,
    pub bottom_image: Image,
    pub body_image: Image,
    pub position: u64,
    pub duration: Option<u64>,
    pub progress_bar: ProgressBar,
    pub scale: Scale,
    pub bottom_position: Label,
    pub body_position: Label,
}

impl NowPlaying {
    fn update_image(&self, other: bool) {
        if let Some(cover) = &self.cover {
            if self.body_image.is_realized() != other { &self.body_image } else { &self.bottom_image }.set_cover(cover);
        }
    }
    fn update_position(&self, other: bool) {
        if self.bottom_position.is_realized() != other {
            self.bottom_position.set_label(&format(self.position));
            if let Some(duration) = self.duration {
                self.progress_bar.set_fraction(self.position as f64 / duration as f64);
            }
        } else {
            self.body_position.set_label(&format(self.position));
            self.scale.set_value(self.position as f64);
        }
    }
    fn update_duration(&self, duration_label: &Label, other: bool) {
        if self.scale.is_realized() != other {
            if let Some(duration) = self.duration {
                self.scale.set_range(0.0, duration as f64);
                duration_label.set_label(&format(duration));
            }
        }
    }
    pub fn set_duration(&mut self, duration_label: &Label) {
        self.duration = PLAYBIN.query_duration().map(ClockTime::nseconds);
        self.update_duration(duration_label, false);
    }
    pub fn update_other(&self, duration_label: &Label, back_button: &Button, icon_name: &str, header_body: &gtk::Box,
        body: &gtk::Box) {
        self.update_image(true);
        self.update_position(true);
        self.update_duration(&duration_label, true);
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
