use std::path::PathBuf;
use std::sync::Arc;
use gstreamer::prelude::ElementExt;
use gstreamer::State::Null;
use mpris_player::MprisPlayer;
use crate::common::constant::APP_ID;
use crate::controls::playbin::{go_delta_song, PLAYBIN, Playbin};

pub(in crate::controls) fn mpris_player() -> Arc<MprisPlayer> {
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
    mpris_player.connect_stop(move || {
        PLAYBIN.set_uri(&PathBuf::from(""));
        PLAYBIN.set_state(Null).unwrap();
    });
    mpris_player.connect_next(|| { go_delta_song(1, true) });
    mpris_player.connect_previous(|| { go_delta_song(-1, true) });
    mpris_player
}
