use once_cell::sync::Lazy;

pub const SONG_SELECTED: &'static str = "song-selected";
pub const STREAM_STARTED: &'static str = "stream-started";

fn window(action_name: &str) -> String {
    format!("win.{}", action_name)
}

pub static WIN_SONG_SELECTED: Lazy<String> = Lazy::new(|| { window(SONG_SELECTED) });
pub static WIN_STREAM_STARTED: Lazy<String> = Lazy::new(|| { window(STREAM_STARTED) });
