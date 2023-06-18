use std::path::Path;
use gtk::{Button, Frame, MediaFile, Orientation};
use gtk::prelude::{BoxExt, ButtonExt, MediaStreamExt};
use Orientation::Horizontal;
use crate::common::gtk_box;

const PLAY_ICON: &'static str = "media-playback-start";

pub fn media_controls() -> Frame {
    let media_file = MediaFile::for_filename(Path::new("/mnt/84ac3f9a-dd17-437d-9aad-5c976e6b81e8/Music/Amorphis/Skyforger-2009/01 - Sampo.mp3"));
    let play_pause = Button::builder().icon_name(PLAY_ICON).build();
    play_pause.connect_clicked(move |play_pause| {
        play_pause.set_icon_name(if media_file.is_playing() {
            media_file.pause();
            PLAY_ICON
        } else {
            media_file.play();
            "media-playback-pause"
        });
    });
    let gtk_box = gtk_box(Horizontal);
    gtk_box.append(&Button::builder().icon_name("media-skip-backward").build());
    gtk_box.append(&play_pause);
    gtk_box.append(&Button::builder().icon_name("media-skip-forward").build());
    Frame::builder().child(&gtk_box).build()
}
