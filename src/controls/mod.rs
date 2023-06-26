mod volume;

use std::rc::Rc;
use std::time::Duration;
use gtk::*;
use gtk::prelude::WidgetExt;
use prelude::{BoxExt, ButtonExt, MediaStreamExt, RangeExt};
use Orientation::Horizontal;
use crate::common::gtk_box;
use crate::controls::volume::volume_button;

enum PlayPause {
    Play,
    Pause,
}

impl PlayPause {
    fn icon_tooltip(&self) -> (&'static str, &'static str) {
        match self {
            PlayPause::Play => ("media-playback-start", "Play"),
            PlayPause::Pause => ("media-playback-pause", "Pause"),
        }
    }
}

fn format(timestamp: i64) -> String {
    let seconds = Duration::from_micros(timestamp as u64).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn media_controls(media_file: Rc<MediaFile>) -> Frame {
    let (icon, tooltip) = PlayPause::Play.icon_tooltip();
    let play_pause = Button::builder().icon_name(icon).tooltip_text(tooltip).build();
    let time = Label::builder().label(format(0)).build();
    let scale = Rc::new(Scale::builder().hexpand(true).build());
    let duration_label = Rc::new(Label::builder().build());
    let gtk_box = gtk_box(Horizontal);
    gtk_box.append(&Button::builder().icon_name("media-skip-backward").tooltip_text("Previous").build());
    gtk_box.append(&play_pause);
    gtk_box.append(&Button::builder().icon_name("media-skip-forward").tooltip_text("Next").build());
    gtk_box.append(&time);
    gtk_box.append(&*scale);
    gtk_box.append(&*duration_label);
    gtk_box.append(&*volume_button({
        let media_file = media_file.clone();
        move |volume| { media_file.set_volume(volume); }
    }));
    media_file.connect_duration_notify({
        let scale = scale.clone();
        move |media_file| {
            let duration = media_file.duration();
            duration_label.set_label(&format(duration));
            scale.set_range(0.0, duration as f64);
        }
    });
    media_file.connect_timestamp_notify({
        let scale = scale.clone();
        move |media_file| {
            let timestamp = media_file.timestamp();
            time.set_label(&format(timestamp));
            scale.set_value(timestamp as f64);
        }
    });
    play_pause.connect_clicked({
        let media_file = media_file.clone();
        move |play_pause| {
            let (icon, tooltip) = if media_file.is_playing() {
                media_file.pause();
                PlayPause::Play
            } else {
                media_file.play();
                PlayPause::Pause
            }.icon_tooltip();
            play_pause.set_icon_name(icon);
            play_pause.set_tooltip_text(Some(tooltip));
        }
    });
    scale.connect_change_value(move |_, scroll_type, value| {
        if scroll_type == ScrollType::Jump {
            media_file.seek(value as i64);
        }
        Inhibit(true)
    });
    Frame::builder().child(&gtk_box).build()
}
