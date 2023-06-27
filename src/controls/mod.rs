mod volume;

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use gtk::*;
use gtk::prelude::*;
use Orientation::Horizontal;
use crate::common::gtk_box;
use crate::common::util::PathString;
use crate::common::wrapper::{SONG_SELECTED, Wrapper};
use crate::controls::volume::volume_button;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::path as song_path;

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

pub fn media_controls() -> Wrapper {
    let path_buf = songs.inner_join(collections).inner_join(config).select((path, song_path))
        .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, current_song_path)| {
        collection_path.to_path().join(current_song_path.to_path())
    }).unwrap_or(PathBuf::from(""));
    let media_file = Rc::new(MediaFile::for_filename(path_buf));
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
    scale.connect_change_value({
        let media_file = media_file.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                media_file.seek(value as i64);
            }
            Inhibit(true)
        }
    });
    let wrapper = Wrapper::new(&Frame::builder().child(&gtk_box).build());
    wrapper.connect_local(SONG_SELECTED, true, move |params| {
        if let [_, song_id, current_song_path, collection_path] = &params {
            update(config).set(current_song_id.eq(song_id.get::<i32>().unwrap())).execute(&mut get_connection()).unwrap();
            let playing = media_file.is_playing();
            if playing { play_pause.emit_clicked(); }
            media_file.set_filename(Some(current_song_path.get::<String>().unwrap().to_path()
                .join(collection_path.get::<String>().unwrap().to_path())));
            // if playing { media_file.play(); }
        }
        None
    });
    wrapper
}
