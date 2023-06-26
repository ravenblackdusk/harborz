use std::rc::Rc;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Frame, Label, ListBox, SelectionMode};
use gtk::prelude::{FrameExt, ObjectExt};
use crate::collection::song::Song;
use crate::db::get_connection;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

const ARTIST: &'static str = "artist";
const ALBUM: &'static str = "album";
const SONG: &'static str = "song";

fn list_box(strings: Vec<Option<String>>, data_key: &str) -> ListBox {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for string in strings {
        let deref_string = string.as_deref();
        let label = Label::builder().label(deref_string.unwrap_or("None")).build();
        unsafe { label.set_data(data_key, deref_string.map(|it| { it.to_owned() })); }
        list_box.append(&label);
    }
    list_box
}

pub fn home() -> Rc<Frame> {
    let artists = list_box(songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection())
        .unwrap(), ARTIST);
    let frame = Rc::new(Frame::builder().child(&artists).build());
    artists.connect_row_activated({
        let frame = frame.clone();
        move |_, artist_row| unsafe {
            let artist_string = gtk::prelude::ListBoxRowExt::child(artist_row).unwrap().data::<Option<String>>(ARTIST)
                .unwrap().as_ref();
            let albums = list_box(songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                .get_results::<Option<String>>(&mut get_connection()).unwrap(), ALBUM);
            albums.connect_row_activated({
                let artist_string = artist_string.to_owned();
                let frame = frame.clone();
                move |_, album_row| {
                    let album_string = gtk::prelude::ListBoxRowExt::child(album_row).unwrap()
                        .data::<Option<String>>(ALBUM).unwrap().as_ref();
                    let song_vec = songs.filter(artist.eq(artist_string.to_owned()).and(album.eq(album_string)))
                        .get_results::<Song>(&mut get_connection()).unwrap();
                    let song_list_box = list_box(song_vec.into_iter().map(|song| { song.title.or(Some(song.path)) })
                        .collect::<Vec<_>>(), SONG);
                    frame.set_child(Some(&song_list_box));
                }
            });
            frame.set_child(Some(&albums));
        }
    });
    frame
}
