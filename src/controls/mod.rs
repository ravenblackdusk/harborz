use std::path::Path;
use std::time::Duration;
use Align::Center;
use gtk::{Align, Button, Frame, Label, MediaFile, Orientation, ProgressBar};
use gtk::prelude::{BoxExt, ButtonExt, MediaStreamExt};
use Orientation::Horizontal;
use crate::common::gtk_box;

const PLAY_ICON: &'static str = "media-playback-start";

fn format(timestamp: i64) -> String {
    let seconds = Duration::from_micros(timestamp as u64).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn media_controls() -> Frame {
    let media_file = MediaFile::for_filename(Path::new("/mnt/84ac3f9a-dd17-437d-9aad-5c976e6b81e8/Music/Amorphis/Skyforger-2009/01 - Sampo.mp3"));
    let play_pause = Button::builder().icon_name(PLAY_ICON).build();
    let time = Label::builder().label(format(0)).build();
    let progress_bar = ProgressBar::builder().valign(Center).build();
    let duration = Label::builder().build();
    let gtk_box = gtk_box(Horizontal);
    gtk_box.append(&Button::builder().icon_name("media-skip-backward").build());
    gtk_box.append(&play_pause);
    gtk_box.append(&Button::builder().icon_name("media-skip-forward").build());
    gtk_box.append(&time);
    gtk_box.append(&progress_bar);
    gtk_box.append(&duration);
    media_file.connect_duration_notify(move |media_file| { duration.set_label(&format(media_file.duration())) });
    media_file.connect_timestamp_notify(move |media_file| {
        let timestamp = media_file.timestamp();
        time.set_label(&format(timestamp));
        progress_bar.set_fraction(timestamp as f64 / media_file.duration() as f64);
    });
    play_pause.connect_clicked(move |play_pause| {
        play_pause.set_icon_name(if media_file.is_playing() {
            media_file.pause();
            PLAY_ICON
        } else {
            media_file.play();
            "media-playback-pause"
        });
    });
    Frame::builder().child(&gtk_box).build()
}
