use once_cell::sync::Lazy;

pub const SONG_SELECTED: &'static str = "song-selected";
pub static WIN_SONG_SELECTED: Lazy<String> = Lazy::new(|| { format!("win.{}", SONG_SELECTED) });
