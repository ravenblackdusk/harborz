use std::rc::Rc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Frame, Label, ListBox, SelectionMode};
use gtk::prelude::{FrameExt, ObjectExt};
use crate::db::get_connection;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::songs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct Song {
    pub id: i32,
    pub path: String,
    pub collection_id: i32,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub datetime: Option<i64>,
    pub genre: Option<String>,
    pub track_number: Option<i32>,
    pub album_artist: Option<String>,
}

const ARTIST: &'static str = "artist";

fn label(string: Option<String>) -> Label {
    Label::builder().label(string.unwrap_or("None".to_string())).build()
}

pub fn home() -> Rc<Frame> {
    let artists = ListBox::builder().selection_mode(SelectionMode::None).build();
    let frame = Rc::new(Frame::builder().child(&artists).build());
    for artist_string in songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap() {
        let label = label(artist_string.clone());
        unsafe { label.set_data(ARTIST, artist_string); }
        artists.append(&label);
    }
    artists.connect_row_activated({
        let frame = frame.clone();
        move |_, list_box_row| unsafe {
            let artist_string = gtk::prelude::ListBoxRowExt::child(list_box_row).unwrap().data::<Option<String>>(ARTIST)
                .unwrap().as_ref();
            let albums = ListBox::builder().selection_mode(SelectionMode::None).build();
            for album_string in songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                .get_results::<Option<String>>(&mut get_connection()).unwrap() {
                albums.append(&label(album_string));
            }
            frame.set_child(Some(&albums));
        }
    });
    frame
}
