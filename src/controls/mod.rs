mod volume;

use std::path::PathBuf;
use std::time::Duration;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use gstreamer::{ClockTime, ElementFactory, Pipeline, SeekFlags};
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::AsyncDone;
use gstreamer::prelude::{Cast, Continue, ElementExt, ElementExtManual, ObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, Inhibit, Label, Scale, ScrollType};
use gtk::prelude::{BoxExt, ButtonExt, RangeExt, WidgetExt};
use once_cell::sync::Lazy;
use gtk::Orientation::Horizontal;
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

fn format(timestamp: u64) -> String {
    let seconds = Duration::from_nanos(timestamp).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

//noinspection SpellCheckingInspection
static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    ElementFactory::make("playbin").build().unwrap().downcast::<Pipeline>().unwrap()
});

//noinspection SpellCheckingInspection
trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn play(&self, label: &Label, scale: &Scale);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property("uri", format!("file:{}", uri.to_str().unwrap()));
    }
    fn play(&self, label: &Label, scale: &Scale) {
        self.set_state(Playing).unwrap();
        let label = label.clone();
        let scale = scale.clone();
        timeout_add_local(Duration::from_millis(200), move || {
            if let Some(position) = PLAYBIN.query_position().map(ClockTime::nseconds) {
                label.set_label(&format(position));
                scale.set_value(position as f64);
            }
            Continue(PLAYBIN.current_state() == Playing || PLAYBIN.pending_state() == Playing)
        });
    }
}

pub fn media_controls() -> Wrapper {
    let path_buf = songs.inner_join(collections).inner_join(config).select((path, song_path))
        .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, current_song_path)| {
        collection_path.to_path().join(current_song_path.to_path())
    }).unwrap_or(PathBuf::from(""));
    PLAYBIN.set_uri(&path_buf);
    let (icon, tooltip) = PlayPause::Play.icon_tooltip();
    let play_pause = Button::builder().icon_name(icon).tooltip_text(tooltip).build();
    let position_label = Label::new(Some(&format(0)));
    let scale = Scale::builder().hexpand(true).build();
    scale.set_range(0.0, 1.0);
    let duration_label = Label::new(Some(&format(0)));
    let gtk_box = gtk_box(Horizontal);
    gtk_box.append(&Button::builder().icon_name("media-skip-backward").tooltip_text("Previous").build());
    gtk_box.append(&play_pause);
    gtk_box.append(&Button::builder().icon_name("media-skip-forward").tooltip_text("Next").build());
    gtk_box.append(&position_label);
    gtk_box.append(&scale);
    gtk_box.append(&duration_label);
    gtk_box.append(&volume_button(|volume| { PLAYBIN.set_property("volume", volume); }));
    play_pause.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |play_pause| {
            let (icon, tooltip) = if PLAYBIN.current_state() == Playing {
                PLAYBIN.set_state(Paused).unwrap();
                PlayPause::Play
            } else {
                PLAYBIN.play(&position_label, &scale);
                PlayPause::Pause
            }.icon_tooltip();
            play_pause.set_icon_name(icon);
            play_pause.set_tooltip_text(Some(tooltip));
        }
    });
    scale.connect_change_value(|_, scroll_type, value| {
        if scroll_type == ScrollType::Jump {
            PLAYBIN.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value as u64))
                .unwrap();
        }
        Inhibit(true)
    });
    let wrapper = Wrapper::new(&gtk_box);
    wrapper.connect_local(SONG_SELECTED, true, {
        let scale = scale.clone();
        move |params| {
            if let [_, song_id, current_song_path, collection_path] = &params {
                update(config).set(current_song_id.eq(song_id.get::<i32>().unwrap())).execute(&mut get_connection())
                    .unwrap();
                let playing = PLAYBIN.current_state() == Playing;
                if playing { PLAYBIN.set_state(Null).unwrap(); }
                PLAYBIN.set_uri(&collection_path.get::<String>().unwrap().to_path()
                    .join(current_song_path.get::<String>().unwrap().to_path()));
                if playing {
                    PLAYBIN.play(&position_label, &scale);
                } else {
                    play_pause.emit_clicked();
                }
            }
            None
        }
    });
    PLAYBIN.bus().unwrap().add_watch_local({
        let scale = scale.clone();
        move |_, message| {
            if let AsyncDone(_) = message.view() {
                if let Some(duration) = PLAYBIN.query_duration().map(ClockTime::nseconds) {
                    duration_label.set_label(&format(duration));
                    scale.set_range(0.0, duration as f64);
                }
            }
            Continue(true)
        }
    }).unwrap();
    wrapper
}
