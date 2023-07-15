use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use adw::gio::File;
use adw::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::min;
use gtk::{ColumnView, ColumnViewColumn, Image, Label, ListItem, NoSelection, Picture, ScrolledWindow, SignalListItemFactory, Widget};
use gtk::ContentFit::Contain;
use gtk::gio::ListStore;
use gtk::glib::BoxedAnyObject;
use crate::collection::model::Collection;
use crate::collection::song::{get_current_album, join_path, WithCover};
use crate::collection::song::Song;
use crate::common::{EllipsizedLabelBuilder, util};
use crate::common::util::format;
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::songs::{album, artist, path as song_path};
use crate::schema::songs::dsl::songs;

fn list_box<T: 'static, S: Fn(Rc<T>, &ListItem) + ?Sized + 'static, F: Fn(Rc<T>) + 'static>(
    history: Option<Rc<RefCell<Vec<Box<dyn AsRef<Widget>>>>>>, scrolled_window: &ScrolledWindow, row_items: Vec<Rc<T>>,
    columns: Vec<(Box<S>, bool)>, on_row_activated: F) {
    let store = ListStore::new(BoxedAnyObject::static_type());
    for row_item in row_items.iter() {
        store.append(&BoxedAnyObject::new(row_item.clone()));
    }
    let column_view = ColumnView::builder().single_click_activate(true).model(&NoSelection::new(Some(store)))
        .show_row_separators(true).build();
    column_view.first_child().unwrap().set_visible(false);
    column_view.connect_activate(move |column_view, row| {
        if let Some(history) = &history {
            (*history).borrow_mut().push(Box::new(column_view.clone()));
        }
        on_row_activated(row_items[row as usize].clone());
    });
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

fn append_label_and_go_next(gtk_box: &gtk::Box, label: &Option<String>, list_item: &ListItem) {
    gtk_box.append(&Label::builder().label(util::or_none(label)).ellipsized().build());
    gtk_box.append(&Image::builder().icon_name("go-next-symbolic").build());
    list_item.set_child(Some(gtk_box));
}

fn artist_row(artist_string: Rc<Option<String>>, list_item: &ListItem) {
    let gtk_box = gtk::Box::builder().build();
    append_label_and_go_next(&gtk_box, &*artist_string, list_item);
}

fn album_row(rc: Rc<(Option<String>, Option<String>, Option<String>)>, list_item: &ListItem) {
    let (album_string, collection_path, album_song_path) = rc.borrow();
    let gtk_box = gtk::Box::builder().build();
    let cover = join_path(&collection_path.clone().unwrap(), &album_song_path.clone().unwrap()).cover();
    let picture_builder = Picture::builder().content_fit(Contain);
    gtk_box.append(&if cover.exists() { picture_builder.file(&File::for_path(cover)) } else { picture_builder }.build());
    append_label_and_go_next(&gtk_box, album_string, list_item);
}

pub fn set_body(scrolled_window: &ScrolledWindow, history: Rc<RefCell<Vec<Box<dyn AsRef<Widget>>>>>,
    media_controls: &Wrapper) {
    list_box(Some(history.clone()), scrolled_window,
        songs.select(artist).group_by(artist).get_results::<Option<String>>(&mut get_connection()).unwrap()
            .into_iter().map(Rc::new).collect::<Vec<_>>(),
        vec![(Box::new(artist_row), true)], {
            let scrolled_window = scrolled_window.clone();
            let media_controls = media_controls.clone();
            move |artist_string| {
                list_box(Some(history.clone()), &scrolled_window, songs.inner_join(collections)
                    .filter(artist.eq(&*artist_string)).group_by(album).select((album, min(path), min(song_path)))
                    .get_results::<(Option<String>, Option<String>, Option<String>)>(&mut get_connection()).unwrap()
                    .into_iter().map(Rc::new).collect::<Vec<_>>(), vec![(Box::new(album_row), true)], {
                    let scrolled_window = scrolled_window.clone();
                    let artist_string = artist_string.clone();
                    let media_controls = media_controls.clone();
                    move |rc| {
                        let (album_string, _, _) = rc.borrow();
                        list_box(None, &scrolled_window,
                            get_current_album(&*artist_string, &album_string, &mut get_connection()).into_iter()
                                .map(|(song, collection)| { Rc::new((media_controls.clone(), song, collection)) })
                                .collect::<Vec<_>>(),
                            vec![
                                (Box::new(|rc: Rc<(Wrapper, Song, Collection)>, list_item: &ListItem| {
                                    let (wrapper, song, _) = &*rc;
                                    let image = Image::builder().width_request(4).build();
                                    let id = song.id;
                                    list_item.set_child(Some(&image));
                                    wrapper.connect_local(STREAM_STARTED, true, move |params| {
                                        image.set_icon_name((id == params[1].get::<i32>().unwrap())
                                            .then_some("media-playback-start"));
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
                                    list_item.set_child(Some(&Label::builder().label(song.title_str()).ellipsized()
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
