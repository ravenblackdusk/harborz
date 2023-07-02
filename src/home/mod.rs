use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Label, ListBox, ScrolledWindow, SelectionMode, Widget};
use gtk::Orientation::Horizontal;
use gtk::prelude::{BoxExt, IsA, ObjectExt};
use gtk::Align::Start;
use crate::collection::model::Collection;
use crate::collection::song::Song;
use crate::common::gtk_box;
use crate::common::util::{format, PathString};
use crate::common::wrapper::{SONG_SELECTED, Wrapper};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::songs::{album, artist, track_number};
use crate::schema::songs::dsl::songs;

const ID: &'static str = "id";

fn list_box<T: 'static, W: IsA<Widget>, S: Fn(&T) -> W, F: Fn(&T) + 'static>(scrolled_window: &ScrolledWindow,
    row_items: Vec<T>, to_widget: S, on_row_activated: F) {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for row_item in row_items {
        let widget = to_widget(&row_item);
        unsafe { widget.set_data(ID, row_item); }
        list_box.append(&widget);
    }
    list_box.connect_row_activated(move |_, list_box_row| {
        let item = unsafe { gtk::prelude::ListBoxRowExt::child(list_box_row).unwrap().data::<T>(ID).unwrap().as_ref() };
        on_row_activated(item);
    });
    scrolled_window.set_child(Some(&list_box));
}

fn or_none(string: &Option<String>) -> Label {
    Label::builder().label(string.as_deref().unwrap_or("None")).halign(Start).build()
}

pub fn set_body(scrolled_window: &ScrolledWindow, media_controls: &Wrapper) {
    list_box(scrolled_window,
        songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap(), or_none, {
            let scrolled_window = scrolled_window.clone();
            let media_controls = media_controls.clone();
            move |artist_string| {
                list_box(&scrolled_window, songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                    .get_results::<Option<String>>(&mut get_connection()).unwrap(), or_none, {
                    let scrolled_window = scrolled_window.clone();
                    let artist_string = artist_string.to_owned();
                    let media_controls = media_controls.clone();
                    move |album_string| {
                        list_box(&scrolled_window,
                            songs.inner_join(collections).filter(artist.eq(&artist_string).and(album.eq(album_string)))
                                .order_by(track_number).get_results::<(Song, Collection)>(&mut get_connection()).unwrap(),
                            |(song, _)| {
                                let artist_box = gtk_box(Horizontal);
                                artist_box.append(&Label::new(song.track_number.map(|it| { it.to_string() }).as_deref()));
                                artist_box.append(&Label::builder().hexpand(true).halign(Start).label(song.title.as_deref()
                                    .unwrap_or(song.path.to_path().file_name().unwrap().to_str().unwrap())).build());
                                artist_box.append(&Label::new(Some(&format(song.duration as u64))));
                                artist_box
                            }, {
                                let media_controls = media_controls.clone();
                                move |(song, collection)| {
                                    media_controls.emit_by_name::<()>(SONG_SELECTED, &[&song.id, &song.path, &collection.path]);
                                }
                            },
                        );
                    }
                });
            }
        },
    );
}
