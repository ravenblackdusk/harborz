use diesel::{QueryDsl, RunQueryDsl};
use gtk::{Label, ListBox, SelectionMode};
use crate::db::get_connection;
use crate::schema::songs::artist;
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

pub(in crate) fn home() -> ListBox {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for artist_string in songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap() {
        list_box.append(&Label::builder().label(artist_string.unwrap_or("None".to_string())).build());
    }
    list_box
}
