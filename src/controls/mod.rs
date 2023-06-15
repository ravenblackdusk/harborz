use gtk::{MediaControls, MediaFile};

pub fn media_controls() -> MediaControls {
    let path = "/mnt/8ff03919-86c0-43c8-acc9-4fdfab52b0f8/My Music/Absu/2001 - Tara/01 - Tara.mp3";
    MediaControls::builder().media_stream(&MediaFile::for_resource(path)).build()
}
