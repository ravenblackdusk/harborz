mod volume;

use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use gtk::*;
use prelude::{BoxExt, ButtonExt, MediaStreamExt, RangeExt};
use Orientation::Horizontal;
use crate::common::gtk_box;
use crate::controls::volume::volume_button;

const PLAY_ICON: &'static str = "media-playback-start";

fn format(timestamp: i64) -> String {
    let seconds = Duration::from_micros(timestamp as u64).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn media_controls() -> Frame {
    let path = "/mnt/8ff03919-86c0-43c8-acc9-4fdfab52b0f8/My Music/Agalloch/The Serpent & the Sphere/08. Plateau Of The Ages.mp3";
    let media_file = Rc::new(MediaFile::for_filename(Path::new(path)));
    let play_pause = Button::builder().icon_name(PLAY_ICON).build();
    let time = Label::builder().label(format(0)).build();
    let scale = Rc::new(Scale::builder().width_request(100).build());
    let duration_label = Rc::new(Label::builder().build());
    let gtk_box = gtk_box(Horizontal);
    gtk_box.append(&Button::builder().icon_name("media-skip-backward").build());
    gtk_box.append(&play_pause);
    gtk_box.append(&Button::builder().icon_name("media-skip-forward").build());
    gtk_box.append(&time);
    gtk_box.append(&*scale);
    gtk_box.append(&*duration_label);
    gtk_box.append(&*volume_button());
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
            play_pause.set_icon_name(if media_file.is_playing() {
                media_file.pause();
                PLAY_ICON
            } else {
                media_file.play();
                "media-playback-pause"
            });
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
