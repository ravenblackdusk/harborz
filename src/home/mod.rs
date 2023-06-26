use std::rc::Rc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Frame, Label, ListBox, SelectionMode};
use gtk::prelude::{FrameExt, ObjectExt};
use crate::db::get_connection;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

const ARTIST: &'static str = "artist";

fn list_box<F: Fn(&Label, Option<&str>)>(strings: Vec<Option<String>>, do_with_label: F) -> ListBox {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for string in strings {
        let deref = string.as_deref();
        let label = Label::builder().label(deref.unwrap_or("None")).build();
        do_with_label(&label, deref);
        list_box.append(&label);
    }
    list_box
}

pub fn home() -> Rc<Frame> {
    let artists = list_box(songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap(),
        |label, string| unsafe { label.set_data(ARTIST, string.map(|it| { it.to_owned() })); });
    let frame = Rc::new(Frame::builder().child(&artists).build());
    artists.connect_row_activated({
        let frame = frame.clone();
        move |_, list_box_row| unsafe {
            let artist_string = gtk::prelude::ListBoxRowExt::child(list_box_row).unwrap().data::<Option<String>>(ARTIST)
                .unwrap().as_ref();
            let albums = list_box(songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                .get_results::<Option<String>>(&mut get_connection()).unwrap(), |_, _| {});
            frame.set_child(Some(&albums));
        }
    });
    frame
}
