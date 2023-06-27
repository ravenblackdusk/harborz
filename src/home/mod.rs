use std::rc::Rc;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Label, ListBox, ScrolledWindow, SelectionMode};
use gtk::prelude::ObjectExt;
use crate::collection::model::Collection;
use crate::collection::song::Song;
use crate::common::wrapper::{SONG_SELECTED, Wrapper};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

const ID: &'static str = "id";

fn list_box<T: 'static, S: Fn(&T) -> &str, F: Fn(&T) + 'static>(scrolled_window: Rc<ScrolledWindow>, row_items: Vec<T>,
    to_str: S, on_row_activated: F) {
    let list_box = ListBox::builder().selection_mode(SelectionMode::None).build();
    for row_item in row_items {
        let label = Label::new(Some(to_str(&row_item)));
        unsafe { label.set_data(ID, row_item); }
        list_box.append(&label);
    }
    list_box.connect_row_activated(move |_, list_box_row| {
        let item = unsafe { gtk::prelude::ListBoxRowExt::child(list_box_row).unwrap().data::<T>(ID).unwrap().as_ref() };
        on_row_activated(item);
    });
    scrolled_window.set_child(Some(&list_box));
}

fn or_none(string: &Option<String>) -> &str {
    string.as_deref().unwrap_or("None")
}

pub fn set_body(scrolled_window: Rc<ScrolledWindow>, media_controls: Rc<Wrapper>) {
    list_box(scrolled_window.clone(),
        songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap(), or_none, {
            let scrolled_window = scrolled_window.clone();
            move |artist_string| {
                list_box(scrolled_window.clone(), songs.filter(artist.eq(artist_string)).select(album).group_by(album)
                    .get_results::<Option<String>>(&mut get_connection()).unwrap(), or_none, {
                    let scrolled_window = scrolled_window.clone();
                    let artist_string = artist_string.to_owned();
                    let media_controls = media_controls.clone();
                    move |album_string| {
                        list_box(scrolled_window.clone(),
                            songs.inner_join(collections)
                                .filter(artist.eq(&artist_string).and(album.eq(album_string)))
                                .get_results::<(Song, Collection)>(&mut get_connection()).unwrap(),
                            |(song, _)| { song.title.as_deref().unwrap_or(song.path.as_str()) }, {
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
