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
use gtk::Orientation::{Horizontal, Vertical};
use gtk::Align::Center;
use log::warn;
use once_cell::sync::Lazy;
use crate::common::{box_builder, gtk_box};
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

const PLAY_ICON: &'static str = "media-playback-start";
const PLAY_TOOLTIP: &'static str = "Play";
const PAUSE_ICON: &'static str = "media-playback-pause";
const PAUSE_TOOLTIP: &'static str = "Pause";

trait Playable {
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
        self.change_state(PLAY_ICON, PLAY_TOOLTIP);
    }
    fn pause(&self) {
        self.change_state(PAUSE_ICON, PAUSE_TOOLTIP);
    }
}

fn format(timestamp: u64) -> String {
    let seconds = Duration::from_nanos(timestamp).as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

const URI: &'static str = "uri";
//noinspection SpellCheckingInspection
static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    ElementFactory::make("playbin").build().unwrap().downcast::<Pipeline>().unwrap()
});

//noinspection SpellCheckingInspection
trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn get_position(&self) -> Option<u64>;
    fn play(&'static self, label: &Label, scale: &Scale) -> anyhow::Result<()>;
    fn seek_internal(&self, value: u64);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property(URI, format!("file:{}", uri.to_str().unwrap()));
    }
    fn get_position(&self) -> Option<u64> {
        PLAYBIN.query_position().map(ClockTime::nseconds)
    }
    fn play(&'static self, label: &Label, scale: &Scale) -> anyhow::Result<()> {
        self.set_state(Playing)?;
        let label = label.clone();
        let scale = scale.clone();
        timeout_add_local(Duration::from_millis(200), move || {
            if let Some(position) = self.get_position() {
                label.set_label(&format(position));
                scale.set_value(position as f64);
            }
            Continue(self.current_state() == Playing || self.pending_state() == Playing)
        });
        Ok(())
    }
    fn seek_internal(&self, value: u64) {
        self.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value)).unwrap();
    }
}

pub fn media_controls() -> Wrapper {
    let path_buf = songs.inner_join(collections).inner_join(config).select((path, song_path))
        .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, current_song_path)| {
        collection_path.to_path().join(current_song_path.to_path())
    }).unwrap_or(PathBuf::from(""));
    PLAYBIN.set_uri(&path_buf);
    let play_pause = Button::new();
    play_pause.play();
    let position_label = Label::new(Some(&format(0)));
    let scale = Scale::builder().hexpand(true).build();
    scale.set_range(0.0, 1.0);
    let duration_label = Label::new(Some(&format(0)));
    let controls = gtk_box(Vertical);
    let position_box = gtk_box(Horizontal);
    let control_box = box_builder().orientation(Horizontal).halign(Center).build();
    position_box.append(&position_label);
    position_box.append(&scale);
    position_box.append(&duration_label);
    control_box.append(&Button::builder().icon_name("media-skip-backward").tooltip_text("Previous").build());
    let seek_backward = Button::builder().icon_name("media-seek-backward").tooltip_text("Seek 10s backward").build();
    seek_backward.connect_clicked(|_| {
        if let Some(position) = PLAYBIN.get_position() {
            PLAYBIN.seek_internal(position - Duration::from_secs(10).as_nanos() as u64);
        }
    });
    control_box.append(&seek_backward);
    control_box.append(&play_pause);
    let seek_forward = Button::builder().icon_name("media-seek-forward").tooltip_text("Seek 30s forward").build();
    seek_forward.connect_clicked(|_| {
        if let Some(position) = PLAYBIN.get_position() {
            PLAYBIN.seek_internal(position + Duration::from_secs(30).as_nanos() as u64);
        }
    });
    control_box.append(&seek_forward);
    control_box.append(&Button::builder().icon_name("media-skip-forward").tooltip_text("Next").build());
    control_box.append(&volume_button(|volume| { PLAYBIN.set_property("volume", volume); }));
    controls.append(&position_box);
    controls.append(&control_box);
    play_pause.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |play_pause| {
            if PLAYBIN.current_state() == Playing {
                PLAYBIN.set_state(Paused).unwrap();
                play_pause.play();
            } else {
                match PLAYBIN.play(&position_label, &scale) {
                    Ok(_) => { play_pause.pause(); }
                    Err(error) => { warn!("error trying to play {} {}", PLAYBIN.property::<String>(URI), error); }
                }
                if let Ok(_) = PLAYBIN.play(&position_label, &scale) {
                    play_pause.pause();
                }
            }
        }
    });
    scale.connect_change_value(|_, scroll_type, value| {
        if scroll_type == ScrollType::Jump { PLAYBIN.seek_internal(value as u64); }
        Inhibit(true)
    });
    let wrapper = Wrapper::new(&controls);
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
                    PLAYBIN.play(&position_label, &scale).unwrap();
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
