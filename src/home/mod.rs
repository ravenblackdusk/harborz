use std::rc::Rc;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Frame, Label, ListBox, SelectionMode};
use gtk::prelude::{FrameExt, ObjectExt};
use crate::collection::song::Song;
use crate::db::get_connection;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

const ID: &'static str = "id";

fn list_box<T: 'static, S: Fn(&T) -> &str, F: Fn(&T) + 'static>(row_items: Vec<T>, to_str: S, on_row_activated: F) -> ListBox {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for row_item in row_items {
        let label = Label::builder().label(to_str(&row_item)).build();
        unsafe { label.set_data(ID, row_item); }
        list_box.append(&label);
    }
    list_box.connect_row_activated(move |_, list_box_row| unsafe {
        let item = gtk::prelude::ListBoxRowExt::child(list_box_row).unwrap().data::<T>(ID).unwrap().as_ref();
        on_row_activated(item);
    });
    list_box
}

fn or_none(string: &Option<String>) -> &str {
    string.as_deref().unwrap_or("None")
}

pub fn home() -> Rc<Frame> {
    let frame = Rc::new(Frame::builder().build());
    let artists = list_box(songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap(),
        or_none, {
            let frame = frame.clone();
            move |artist_string| {
                let albums = list_box(songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                    .get_results::<Option<String>>(&mut get_connection()).unwrap(), or_none, {
                    let artist_string = artist_string.to_owned();
                    let frame = frame.clone();
                    move |album_string| {
                        let song_list_box = list_box(songs.filter(artist.eq(&artist_string).and(album.eq(album_string)))
                            .get_results::<Song>(&mut get_connection()).unwrap(), |song_item| {
                            song_item.title.as_deref().unwrap_or(song_item.path.as_str())
                        }, move |song_item| {});
                        frame.set_child(Some(&song_list_box));
                    }
                });
                frame.set_child(Some(&albums));
            }
        });
    frame.set_child(Some(&artists));
    frame
}
