use std::ops::Deref;
use std::rc::Rc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{ColumnView, ColumnViewColumn, Image, Label, ListItem, NoSelection, ScrolledWindow, SignalListItemFactory};
use gtk::gio::ListStore;
use gtk::glib::BoxedAnyObject;
use gtk::pango::EllipsizeMode;
use gtk::prelude::{Cast, CastNone, ObjectExt, StaticType, WidgetExt};
use crate::collection::model::Collection;
use crate::collection::song::get_current_album;
use crate::collection::song::Song;
use crate::common::util::{format, PathString};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::db::get_connection;
use crate::schema::songs::{album, artist};
use crate::schema::songs::dsl::songs;

fn list_box<T: 'static, S: Fn(Rc<T>, &ListItem) + ?Sized + 'static, F: Fn(Rc<T>) + 'static>(
    scrolled_window: &ScrolledWindow, row_items: Vec<Rc<T>>, columns: Vec<(Box<S>, bool)>, on_row_activated: F) {
    let store = ListStore::new(BoxedAnyObject::static_type());
    for row_item in row_items.iter() {
        store.append(&BoxedAnyObject::new(row_item.clone()));
    }
    let column_view = ColumnView::builder().single_click_activate(true).model(&NoSelection::new(Some(store))).build();
    column_view.first_child().unwrap().set_visible(false);
    column_view.connect_activate(move |_, row| { on_row_activated(row_items[row as usize].clone()); });
    for (set_child, expand) in columns {
        let item_factory = SignalListItemFactory::new();
        item_factory.connect_bind(move |_, item| {
            let list_item = item.downcast_ref::<ListItem>().unwrap();
            list_item.set_selectable(false);
            set_child(list_item.item().and_downcast::<BoxedAnyObject>().unwrap().borrow::<Rc<T>>().deref().clone(),
                list_item);
        });
        column_view.append_column(&ColumnViewColumn::builder().factory(&item_factory).expand(expand).build());
    }
    scrolled_window.set_child(Some(&column_view));
}

fn or_none(string: Rc<Option<String>>, list_item: &ListItem) {
    list_item.set_child(Some(&Label::builder().label(string.as_deref().unwrap_or("None"))
        .hexpand(true).xalign(0.0).max_width_chars(1).ellipsize(EllipsizeMode::End).build()));
}

pub fn set_body(scrolled_window: &ScrolledWindow, media_controls: &Wrapper) {
    list_box(scrolled_window,
        songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap()
            .into_iter().map(Rc::new).collect::<Vec<_>>(),
        vec![(Box::new(or_none), true)], {
            let scrolled_window = scrolled_window.clone();
            let media_controls = media_controls.clone();
            move |artist_string| {
                list_box(&scrolled_window, songs.filter(artist.eq(&*artist_string)).select(album).group_by(album)
                    .get_results::<Option<String>>(&mut get_connection()).unwrap()
                    .into_iter().map(Rc::new).collect::<Vec<_>>(), vec![(Box::new(or_none), true)], {
                    let scrolled_window = scrolled_window.clone();
                    let artist_string = artist_string.clone();
                    let media_controls = media_controls.clone();
                    move |album_string| {
                        list_box(&scrolled_window,
                            get_current_album(&*artist_string, &*album_string, &mut get_connection()).into_iter()
                                .map(|(song, collection)| { Rc::new((media_controls.clone(), song, collection)) })
                                .collect::<Vec<_>>(),
                            vec![
                                (Box::new(|rc: Rc<(Wrapper, Song, Collection)>, list_item: &ListItem| {
                                    let (wrapper, song, _) = &*rc;
                                    let image = Image::new();
                                    let id = song.id;
                                    list_item.set_child(Some(&image));
                                    wrapper.connect_local(STREAM_STARTED, true, move |params| {
                                        image.set_icon_name(if id == params[1].get::<i32>().unwrap() {
                                            Some("media-playback-start")
                                        } else {
                                            None
                                        });
                                        None
                                    });
                                }) as Box<dyn Fn(Rc<(Wrapper, Song, Collection)>, &ListItem)>, false),
                                (Box::new(|rc, list_item| {
                                    let (_, song, _) = &*rc;
                                    list_item.set_child(Some(&Label::new(song.track_number.map(|it| { it.to_string() })
                                        .as_deref())));
                                }), false),
                                (Box::new(|rc, list_item| {
                                    let (_, song, _) = &*rc;
                                    list_item.set_child(Some(&Label::builder().label(
                                        song.title.as_deref().unwrap_or(song.path.to_path().file_name().unwrap()
                                            .to_str().unwrap())
                                    ).hexpand(true).xalign(0.0).max_width_chars(1).ellipsize(EllipsizeMode::End)
                                        .build()));
                                }), true),
                                (Box::new(|rc, list_item| {
                                    let (_, song, _) = &*rc;
                                    list_item.set_child(Some(&Label::new(Some(&format(song.duration as u64)))));
                                }), false),
                            ], {
                                let media_controls = media_controls.clone();
                                move |rc| {
                                    let (_, song, collection) = &*rc;
                                    media_controls.emit_by_name::<()>(SONG_SELECTED, &[&song.path, &collection.path]);
                                }
                            },
                        );
                    }
                });
            }
        },
    );
}
