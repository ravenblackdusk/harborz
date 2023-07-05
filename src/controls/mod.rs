mod volume;

use std::path::PathBuf;
use std::time::Duration;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, TextExpressionMethods, update};
use gstreamer::{ClockTime, ElementFactory, Pipeline, SeekFlags};
use gstreamer::glib::timeout_add_local;
use gstreamer::MessageView::{AsyncDone, DurationChanged, StateChanged, StreamStart};
use gstreamer::prelude::{Cast, Continue, ElementExt, ElementExtManual, ObjectExt};
use gstreamer::State::{Null, Paused, Playing};
use gtk::{Button, Inhibit, Label, Scale, ScrollType};
use gtk::prelude::{BoxExt, ButtonExt, RangeExt, WidgetExt};
use gtk::Orientation::{Horizontal, Vertical};
use gtk::Align::Center;
use log::warn;
use mpris_player::{Metadata, MprisPlayer, PlaybackStatus};
use once_cell::sync::Lazy;
use song::get_current_album;
use util::format;
use crate::collection::model::Collection;
use crate::collection::song;
use crate::collection::song::Song;
use crate::common::{box_builder, gtk_box, util};
use crate::common::util::PathString;
use crate::common::wrapper::{SONG_SELECTED, Wrapper};
use crate::controls::volume::volume_button;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::current_song_id;
use crate::schema::config::dsl::config;
use crate::schema::songs::dsl::songs;
use crate::schema::songs::{album, artist, path as song_path};
use crate::common::constant::APP_ID;

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
        self.change_state("media-playback-start", "Play");
    }
    fn pause(&self) {
        self.change_state("media-playback-pause", "Pause");
    }
}

const URI: &'static str = "uri";
//noinspection SpellCheckingInspection
static PLAYBIN: Lazy<Pipeline> = Lazy::new(|| {
    ElementFactory::make("playbin3").build().unwrap().downcast::<Pipeline>().unwrap()
});

//noinspection SpellCheckingInspection
trait Playbin {
    fn set_uri(&self, uri: &PathBuf);
    fn get_position(&self) -> Option<u64>;
    fn seek_internal(&self, value: u64, label: &Label, scale: &Scale) -> anyhow::Result<()>;
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, scale: &Scale);
}

impl Playbin for Pipeline {
    fn set_uri(&self, uri: &PathBuf) {
        self.set_property(URI, format!("file:{}", uri.to_str().unwrap()));
    }
    fn get_position(&self) -> Option<u64> {
        PLAYBIN.query_position().map(ClockTime::nseconds)
    }
    fn seek_internal(&self, value: u64, label: &Label, scale: &Scale) -> anyhow::Result<()> {
        self.seek_simple(SeekFlags::FLUSH | SeekFlags::KEY_UNIT, ClockTime::from_nseconds(value))?;
        label.set_label(&format(value));
        Ok(scale.set_value(value as f64))
    }
    fn simple_seek(&self, duration: Duration, forward: bool, label: &Label, scale: &Scale) {
        if let Some(position) = self.get_position() {
            let nanos = duration.as_nanos() as i64;
            self.seek_internal(
                ((position as i64) + if forward { nanos } else { -nanos })
                    .clamp(0, PLAYBIN.query_duration().map(ClockTime::nseconds).unwrap() as i64) as u64,
                &label, &scale,
            ).unwrap();
        }
    }
}

fn go_delta_song(delta: i32, now: bool) {
    get_connection().transaction(|connection| {
        if let Ok((Some(current_song_id_int), artist_string, album_string)) = config.inner_join(songs)
            .select((current_song_id, artist, album))
            .get_result::<(Option<i32>, Option<String>, Option<String>)>(connection) {
            let song_collections = get_current_album(&artist_string, &album_string, connection);
            let delta_song_index = song_collections.iter().position(|(song, _)| { song.id == current_song_id_int })
                .unwrap() as i32 + delta;
            if delta_song_index >= 0 && delta_song_index < song_collections.len() as i32 {
                let (delta_song, delta_collection) = &song_collections[delta_song_index as usize];
                let playing = PLAYBIN.current_state() == Playing;
                if now { PLAYBIN.set_state(Null).unwrap(); }
                PLAYBIN.set_uri(&delta_collection.path.to_path().join(delta_song.path.to_path()));
                if now && playing { PLAYBIN.set_state(Playing).unwrap(); }
            }
        }
        anyhow::Ok(())
    }).unwrap();
}

pub fn media_controls() -> Wrapper {
    let path_buf = songs.inner_join(collections).inner_join(config).select((path, song_path))
        .get_result::<(String, String)>(&mut get_connection()).map(|(collection_path, current_song_path)| {
        collection_path.to_path().join(current_song_path.to_path())
    }).unwrap_or(PathBuf::from(""));
    PLAYBIN.set_uri(&path_buf);
    let mpris_player = MprisPlayer::new("harborz".to_string(), "Harborz".to_string(), APP_ID.to_string());
    mpris_player.set_can_quit(false);
    mpris_player.set_can_control(false);
    mpris_player.set_can_raise(false);
    mpris_player.set_can_play(true);
    mpris_player.set_can_pause(true);
    mpris_player.set_can_seek(true);
    mpris_player.set_can_go_next(true);
    mpris_player.set_can_go_previous(true);
    mpris_player.set_can_set_fullscreen(false);
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
    let skip_backward = Button::builder().icon_name("media-skip-backward").tooltip_text("Previous").build();
    skip_backward.connect_clicked(|_| { go_delta_song(-1, true); });
    control_box.append(&skip_backward);
    let seek_backward = Button::builder().icon_name("media-seek-backward").tooltip_text("Seek 10s backward").build();
    seek_backward.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |_| { PLAYBIN.simple_seek(Duration::from_secs(10), false, &position_label, &scale); }
    });
    control_box.append(&seek_backward);
    control_box.append(&play_pause);
    let seek_forward = Button::builder().icon_name("media-seek-forward").tooltip_text("Seek 30s forward").build();
    seek_forward.connect_clicked({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |_| { PLAYBIN.simple_seek(Duration::from_secs(30), true, &position_label, &scale); }
    });
    control_box.append(&seek_forward);
    let skip_forward = Button::builder().icon_name("media-skip-forward").tooltip_text("Next").build();
    skip_forward.connect_clicked(|_| { go_delta_song(1, true); });
    control_box.append(&skip_forward);
    control_box.append(&volume_button(|volume| { PLAYBIN.set_property("volume", volume); }));
    controls.append(&position_box);
    controls.append(&control_box);
    play_pause.connect_clicked(move |play_pause| {
        if PLAYBIN.current_state() == Playing {
            PLAYBIN.set_state(Paused).unwrap();
            play_pause.play();
        } else {
            match PLAYBIN.set_state(Playing) {
                Ok(_) => { play_pause.pause(); }
                Err(error) => { warn!("error trying to play {} {}", PLAYBIN.property::<String>(URI), error); }
            }
        }
    });
    scale.connect_change_value({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                if let Err(error) = PLAYBIN.seek_internal(value as u64, &position_label, &scale) {
                    warn!("error trying to seek to {} {}", value, error);
                }
            }
            Inhibit(true)
        }
    });
    let wrapper = Wrapper::new(&controls);
    wrapper.connect_local(SONG_SELECTED, true, {
        let play_pause = play_pause.clone();
        move |params| {
            if let [_, current_song_path, collection_path] = &params {
                let playing = PLAYBIN.current_state() == Playing;
                PLAYBIN.set_state(Null).unwrap();
                PLAYBIN.set_uri(&collection_path.get::<String>().unwrap().to_path()
                    .join(current_song_path.get::<String>().unwrap().to_path()));
                if playing {
                    PLAYBIN.set_state(Playing).unwrap();
                } else {
                    play_pause.emit_clicked();
                }
            }
            None
        }
    });
    mpris_player.connect_play_pause(move || { play_pause.emit_clicked(); });
    mpris_player.connect_next(|| { go_delta_song(1, true) });
    mpris_player.connect_previous(|| { go_delta_song(-1, true) });
    mpris_player.connect_seek({
        let position_label = position_label.clone();
        let scale = scale.clone();
        move |delta_micros| {
            PLAYBIN.simple_seek(Duration::from_micros(delta_micros.abs() as u64), delta_micros >= 0, &position_label,
                &scale);
        }
    });
    PLAYBIN.connect("about-to-finish", true, move |_| {
        go_delta_song(1, false);
        None
    });
    PLAYBIN.bus().unwrap().add_watch_local({
        let scale = scale.clone();
        move |_, message| {
            match message.view() {
                StateChanged(state_changed) => {
                    match state_changed.current() {
                        Playing => {
                            mpris_player.set_playback_status(PlaybackStatus::Playing);
                            timeout_add_local(Duration::from_millis(200), {
                                let position_label = position_label.clone();
                                let scale = scale.clone();
                                move || {
                                    if let Some(position) = PLAYBIN.get_position() {
                                        position_label.set_label(&format(position));
                                        scale.set_value(position as f64);
                                    }
                                    Continue(PLAYBIN.current_state() == Playing || PLAYBIN.pending_state() == Playing)
                                }
                            });
                        }
                        Paused => { mpris_player.set_playback_status(PlaybackStatus::Paused); }
                        _ => {}
                    }
                }
                AsyncDone(_) | DurationChanged(_) => {
                    if let Some(duration) = PLAYBIN.query_duration().map(ClockTime::nseconds) {
                        duration_label.set_label(&format(duration));
                        scale.set_range(0.0, duration as f64);
                    }
                }
                StreamStart(_) => {
                    let uri = &PLAYBIN.property::<String>("current-uri")[5.. /* remove "file:" */];
                    get_connection().transaction(|connection| {
                        let (_, song) = collections.inner_join(songs)
                            .filter(path.concat("/").concat(song_path).eq(uri))
                            .get_result::<(Collection, Song)>(connection)?;
                        update(config).set(current_song_id.eq(song.id)).execute(connection)?;
                        mpris_player.set_metadata(Metadata {
                            length: Some(song.duration),
                            art_url: None,
                            album: song.album,
                            album_artist: song.album_artist.map(|it| { vec![it] }),
                            artist: song.artist.map(|it| { vec![it] }),
                            composer: None,
                            disc_number: None,
                            genre: song.genre.map(|it| { vec![it] }),
                            title: song.title.or(song.path.to_path().file_name().unwrap().to_str()
                                .map(|it| { it.to_string() })),
                            track_number: song.track_number,
                            url: Some(uri.to_string()),
                        });
                        anyhow::Ok(())
                    }).unwrap();
                }
                _ => {}
            }
            Continue(true)
        }
    }).unwrap();
    wrapper
}
