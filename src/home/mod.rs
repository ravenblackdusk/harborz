use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use adw::gdk::gdk_pixbuf::Pixbuf;
use adw::gdk::pango::{AttrInt, AttrList};
use adw::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::{count_distinct, count_star, min};
use gtk::{ColumnView, ColumnViewColumn, Image, Label, ListItem, NoSelection, Picture, ScrolledWindow, SignalListItemFactory, Widget};
use gtk::ContentFit::Contain;
use gtk::gio::ListStore;
use gtk::glib::BoxedAnyObject;
use gtk::Orientation::Vertical;
use gtk::pango::Weight;
use crate::collection::model::Collection;
use crate::collection::song::{get_current_album, join_path, WithCover};
use crate::collection::song::Song;
use crate::common::{BoldLabelBuilder, EllipsizedLabelBuilder, SubscriptLabelBuilder, util};
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

fn or_none_label(label: &Option<String>) -> Label {
    Label::builder().label(util::or_none(label)).margin_ellipsized(4).bold().build()
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").build()
}

fn bold_if_now_playing(song: &Song, list_item: &ListItem, label: Label, wrapper: &Wrapper) {
    let id = song.id;
    list_item.set_child(Some(&label));
    wrapper.connect_local(STREAM_STARTED, true, move |params| {
        let attr_list = label.attributes().unwrap_or_else(AttrList::new);
        label.set_attributes(Some(&if id == params[1].get::<i32>().unwrap() {
            label.add_css_class("accent");
            attr_list.change(AttrInt::new_weight(Weight::Bold));
            attr_list
        } else {
            label.remove_css_class("accent");
            attr_list.change(AttrInt::new_weight(Weight::Normal));
            attr_list
        }));
        None
    });
}

pub fn set_body(scrolled_window: &ScrolledWindow, history: Rc<RefCell<Vec<Box<dyn AsRef<Widget>>>>>,
    media_controls: &Wrapper) {
    list_box(Some(history.clone()), scrolled_window,
        songs.group_by(artist).select((artist, count_distinct(album), count_star()))
            .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap().into_iter().map(Rc::new)
            .collect::<Vec<_>>(),
        vec![(Box::new(|rc: Rc<(Option<String>, i64, i64)>, list_item: &ListItem| {
            let (album_string, album_count, song_count) = rc.borrow();
            let artist_row = gtk::Box::builder().margin_top(4).margin_bottom(4).build();
            let artist_box = gtk::Box::builder().orientation(Vertical).build();
            let count_box = gtk::Box::builder().spacing(4).build();
            let album_count_box = gtk::Box::builder().spacing(4).build();
            let song_count_box = gtk::Box::builder().spacing(4).build();
            count_box.append(&album_count_box);
            count_box.append(&song_count_box);
            album_count_box.append(&Label::builder().label(album_count.to_string()).margin_start(4).subscript().build());
            album_count_box.append(&Label::builder().label("Albums").subscript().build());
            song_count_box.append(&Label::builder().label(song_count.to_string()).subscript().build());
            song_count_box.append(&Label::builder().label("Songs").subscript().build());
            artist_box.append(&or_none_label(album_string));
            artist_box.append(&count_box);
            artist_row.append(&artist_box);
            artist_row.append(&next_icon());
            list_item.set_child(Some(&artist_row));
        }), true)], {
            let scrolled_window = scrolled_window.clone();
            let media_controls = media_controls.clone();
            move |rc| {
                let (artist_string, _, _) = rc.borrow();
                list_box(Some(history.clone()), &scrolled_window, songs.inner_join(collections)
                    .filter(artist.eq(artist_string)).group_by(album)
                    .select((album, count_star(), min(path), min(song_path)))
                    .get_results::<(Option<String>, i64, Option<String>, Option<String>)>(&mut get_connection())
                    .unwrap().into_iter().map(Rc::new).collect::<Vec<_>>(),
                    vec![
                        (Box::new(|rc: Rc<(Option<String>, i64, Option<String>, Option<String>)>, list_item: &ListItem| {
                            let (_, _, collection_path, album_song_path) = rc.borrow();
                            let cover = join_path(&collection_path.clone().unwrap(), &album_song_path.clone().unwrap())
                                .cover();
                            if cover.exists() {
                                let picture = Picture::builder().content_fit(Contain).build();
                                picture.set_pixbuf(Some(&Pixbuf::from_file_at_scale(cover, -1, 70, true).unwrap()));
                                list_item.set_child(Some(&picture));
                            }
                        }) as Box<dyn Fn(Rc<(Option<String>, i64, Option<String>, Option<String>)>, &ListItem)>, false),
                        (Box::new(|rc, list_item| {
                            let (album_string, count, _, _) = rc.borrow();
                            let album_row = gtk::Box::builder().margin_top(4).margin_bottom(4).build();
                            let album_box = gtk::Box::builder().orientation(Vertical).build();
                            let count_box = gtk::Box::builder().spacing(4).build();
                            album_box.append(&or_none_label(album_string));
                            count_box.append(&Label::builder().label(count.to_string()).margin_start(4).subscript()
                                .build());
                            count_box.append(&Label::builder().label("Songs").subscript().build());
                            album_box.append(&count_box);
                            album_row.append(&album_box);
                            album_row.append(&next_icon());
                            list_item.set_child(Some(&album_row));
                        }), true),
                    ], {
                        let scrolled_window = scrolled_window.clone();
                        let artist_string = artist_string.clone();
                        let media_controls = media_controls.clone();
                        move |rc| {
                            let (album_string, _, _, _) = rc.borrow();
                            list_box(None, &scrolled_window,
                                get_current_album(&artist_string, &album_string, &mut get_connection()).into_iter()
                                    .map(|(song, collection)| { Rc::new((media_controls.clone(), song, collection)) })
                                    .collect::<Vec<_>>(),
                                vec![
                                    (Box::new(|rc: Rc<(Wrapper, Song, Collection)>, list_item: &ListItem| {
                                        let (wrapper, song, _) = &*rc;
                                        let label = Label::new(song.track_number.map(|it| { it.to_string() })
                                            .as_deref());
                                        bold_if_now_playing(song, list_item, label, wrapper);
                                    }) as Box<dyn Fn(Rc<(Wrapper, Song, Collection)>, &ListItem)>, false),
                                    (Box::new(|rc, list_item| {
                                        let (wrapper, song, _) = &*rc;
                                        let label = Label::builder().label(song.title_str()).ellipsized().build();
                                        bold_if_now_playing(song, list_item, label, wrapper);
                                    }), true),
                                    (Box::new(|rc, list_item| {
                                        let (wrapper, song, _) = &*rc;
                                        let label = Label::builder().label(&format(song.duration as u64)).subscript()
                                            .build();
                                        bold_if_now_playing(song, list_item, label, wrapper);
                                    }), false),
                                ], {
                                    let media_controls = media_controls.clone();
                                    move |rc| {
                                        let (_, song, collection) = &*rc;
                                        media_controls.emit_by_name::<()>(SONG_SELECTED,
                                            &[&song.path, &collection.path]);
                                    }
                                },
                            );
                        }
                    },
                );
            }
        },
    );
}
