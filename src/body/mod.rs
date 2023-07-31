use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use adw::{ApplicationWindow, WindowTitle};
use adw::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::{count_distinct, count_star, min};
use gtk::{Button, ColumnView, ColumnViewColumn, Image, Label, ListItem, NoSelection, ScrolledWindow, SignalListItemFactory, Widget};
use gtk::gio::ListStore;
use gtk::glib::BoxedAnyObject;
use gtk::Orientation::Vertical;
use crate::body::collection::add_collection_box;
use crate::body::collection::model::Collection;
use crate::common::{AdjustableScrolledWindow, BoldLabelBuilder, BoldSubscriptLabelBuilder, EllipsizedLabelBuilder, SubscriptLabelBuilder};
use crate::common::constant::UNKNOWN_ALBUM;
use crate::common::util::{format, or_none, or_none_static};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist, path as song_path};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, join_path, WithCover};
use crate::song::Song;

pub mod collection;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::bodies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BodyTable {
    pub id: i32,
    pub query1: Option<String>,
    pub query2: Option<String>,
    pub body_type: BodyType,
    pub scroll_adjustment: Option<f32>,
    pub navigation_type: NavigationType,
}

#[derive(Debug, PartialEq, diesel_derive_enum::DbEnum)]
pub enum BodyType {
    Artists,
    Albums,
    Songs,
    Collections,
}

#[derive(Debug, diesel_derive_enum::DbEnum)]
pub enum NavigationType {
    History,
    SongSelected,
}

pub struct Body {
    pub title: Rc<String>,
    pub subtitle: Rc<String>,
    pub body_type: BodyType,
    pub query1: Option<Rc<String>>,
    pub query2: Option<Rc<String>>,
    pub scroll_adjustment: Cell<Option<f32>>,
    pub widget: Box<dyn AsRef<Widget>>,
}

fn column_view<T: 'static, S: Fn(Rc<T>, &ListItem) + ?Sized + 'static, F: Fn(Rc<T>) + 'static>(row_items: Vec<Rc<T>>,
    columns: Vec<(Box<dyn Fn() -> Box<dyn AsRef<Widget>>>, Box<S>, bool)>, on_row_activated: F) -> ColumnView {
    let store = ListStore::new(BoxedAnyObject::static_type());
    for row_item in row_items.iter() {
        store.append(&BoxedAnyObject::new(row_item.clone()));
    }
    let column_view = ColumnView::builder().single_click_activate(true).model(&NoSelection::new(Some(store)))
        .show_row_separators(true).build();
    column_view.first_child().unwrap().set_visible(false);
    column_view.connect_activate(move |_, row| { on_row_activated(row_items[row as usize].clone()); });
    for (setup_child, bind, expand) in columns {
        let item_factory = SignalListItemFactory::new();
        item_factory.connect_setup(move |_, item| {
            let list_item = item.downcast_ref::<ListItem>().unwrap();
            list_item.set_selectable(false);
            list_item.set_child(Some((*(*setup_child)()).as_ref()));
        });
        item_factory.connect_bind(move |_, item| {
            let list_item = item.downcast_ref::<ListItem>().unwrap();
            bind(list_item.item().and_downcast::<BoxedAnyObject>().unwrap().borrow::<Rc<T>>().deref().clone(),
                list_item);
        });
        column_view.append_column(&ColumnViewColumn::builder().factory(&item_factory).expand(expand).build());
    }
    column_view
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").build()
}

fn accent_if_now_playing(label: &Label, row_id: i32, current_song_id: i32) {
    if row_id == current_song_id {
        label.add_css_class("accent");
    } else {
        label.remove_css_class("accent");
    }
}

fn connect_accent_if_now_playing(song: &Song, current_song_id: Option<i32>, label: Label, wrapper: &Wrapper) {
    let id = song.id;
    if let Some(current_song_id) = current_song_id {
        accent_if_now_playing(&label, id, current_song_id);
    }
    wrapper.connect_local(STREAM_STARTED, true, move |params| {
        accent_if_now_playing(&label, id, params[1].get::<i32>().unwrap());
        None
    });
}

trait Castable {
    fn first_child(self) -> Option<Widget>;
    fn set_label(self, label: &str) -> Label;
}

impl Castable for Option<Widget> {
    fn first_child(self) -> Option<Widget> {
        self.and_downcast::<gtk::Box>().unwrap().first_child()
    }
    fn set_label(self, label: &str) -> Label {
        let result = self.and_downcast::<Label>().unwrap();
        result.set_label(label);
        result
    }
}

impl Body {
    pub fn from_body_table(body_table: &BodyTable, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, media_controls: &Wrapper, back_button: &Button,
        window: &ApplicationWindow) -> Self {
        match body_table.body_type {
            BodyType::Artists => {
                Body::artists(&window_title, &scrolled_window, history.clone(), &media_controls,
                    &Some(back_button.clone()))
            }
            BodyType::Albums => {
                Body::albums(body_table.query1.clone(), &window_title, &scrolled_window, history.clone(),
                    &media_controls)
            }
            BodyType::Songs => {
                Body::songs(body_table.query1.clone(), body_table.query2.clone().map(Rc::new), &media_controls)
            }
            BodyType::Collections => { Body::collections(&window) }
        }
    }
    pub fn put_to_history(self, scroll_adjustment: Option<f32>, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) {
        self.scroll_adjustment.set(scroll_adjustment);
        history.borrow_mut().push((Rc::new(self), true));
    }
    pub fn set(self: Rc<Self>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Option<Button>) {
        if let Some(back_button) = back_button { back_button.set_visible(true); }
        window_title.set_title(&self.title);
        window_title.set_subtitle(&self.subtitle);
        let mut history = history.borrow_mut();
        if let Some((body, _)) = history.last() {
            let Body { scroll_adjustment, .. } = body.deref();
            scroll_adjustment.set(scrolled_window.get_adjustment());
        }
        scrolled_window.set_child(Some((*self.widget).as_ref()));
        history.push((self, false));
    }
    pub fn collections(window: &ApplicationWindow) -> Self {
        Self {
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Collection")),
            body_type: BodyType::Collections,
            query1: None,
            query2: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new(add_collection_box(&window)),
        }
    }
    pub fn artists(window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, media_controls: &Wrapper, back_button: &Option<Button>) -> Self {
        Self {
            body_type: BodyType::Artists,
            query1: None,
            query2: None,
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Artists")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(
                column_view(
                    songs.group_by(artist).select((artist, count_distinct(album), count_star()))
                        .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap().into_iter()
                        .map(Rc::new).collect::<Vec<_>>(),
                    vec![(Box::new(|| {
                        let artist_row = gtk::Box::builder().margin_top(4).margin_bottom(4).build();
                        let artist_box = gtk::Box::builder().orientation(Vertical).build();
                        artist_row.append(&artist_box);
                        artist_box.append(&Label::builder().margin_ellipsized(4).bold().build());
                        let count_box = gtk::Box::builder().spacing(4).build();
                        artist_box.append(&count_box);
                        let album_count_box = gtk::Box::builder().spacing(4).build();
                        count_box.append(&album_count_box);
                        album_count_box.append(&Label::builder().margin_start(4).subscript().build());
                        album_count_box.append(&Label::builder().label("Albums").subscript().build());
                        let song_count_box = gtk::Box::builder().spacing(4).build();
                        count_box.append(&song_count_box);
                        song_count_box.append(&Label::builder().subscript().build());
                        song_count_box.append(&Label::builder().label("Songs").subscript().build());
                        artist_row.append(&next_icon());
                        Box::new(artist_row)
                    }), Box::new(|rc: Rc<(Option<String>, i64, i64)>, list_item: &ListItem| {
                        let (album_string, album_count, song_count) = rc.borrow();
                        let artist_box = list_item.child().first_child().and_downcast::<gtk::Box>().unwrap();
                        artist_box.first_child().set_label(or_none(album_string));
                        let count_box = artist_box.last_child().and_downcast::<gtk::Box>().unwrap();
                        count_box.first_child().first_child().set_label(&album_count.to_string());
                        count_box.last_child().first_child().set_label(&song_count.to_string());
                    }), true)], {
                        let window_title = window_title.clone();
                        let scrolled_window = scrolled_window.clone();
                        let media_controls = media_controls.clone();
                        let back_button = back_button.clone();
                        move |rc| {
                            let (artist_string, _, _) = rc.borrow();
                            Rc::new(Self::albums(artist_string.clone(), &window_title, &scrolled_window, history.clone(),
                                &media_controls,
                            )).set(&window_title, &scrolled_window, history.clone(), &back_button);
                        }
                    },
                )
            ),
        }
    }
    pub fn albums(artist_string: Option<String>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, media_controls: &Wrapper) -> Self {
        let artist_string = artist_string.map(Rc::new);
        Self {
            body_type: BodyType::Albums,
            query1: artist_string.clone(),
            query2: None,
            title: or_none_static(artist_string.clone()),
            subtitle: Rc::new(String::from("Albums")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(
                column_view(
                    songs.inner_join(collections)
                        .filter(artist.eq(artist_string.as_deref())).group_by(album)
                        .select((album, count_star(), min(path), min(song_path)))
                        .get_results::<(Option<String>, i64, Option<String>, Option<String>)>(&mut get_connection())
                        .unwrap().into_iter().map(Rc::new).collect::<Vec<_>>(),
                    vec![
                        (Box::new(|| { Box::new(Image::builder().pixel_size(38).build()) }),
                            Box::new(move |rc: Rc<(Option<String>, i64, Option<String>, Option<String>)>,
                                list_item: &ListItem| {
                                let (_, _, collection_path, album_song_path) = rc.borrow();
                                let cover = join_path(&collection_path.clone().unwrap(),
                                    &album_song_path.clone().unwrap()).cover();
                                let image = list_item.child().and_downcast::<Image>().unwrap();
                                if cover.exists() {
                                    image.set_from_file(Some(cover));
                                } else {
                                    image.set_icon_name(Some(UNKNOWN_ALBUM));
                                }
                            }) as Box<dyn Fn(Rc<(Option<String>, i64, Option<String>, Option<String>)>, &ListItem)>,
                            false
                        ), (Box::new(|| {
                            let album_row = gtk::Box::builder().margin_top(4).margin_bottom(4).build();
                            let album_box = gtk::Box::builder().orientation(Vertical).build();
                            album_row.append(&album_box);
                            album_box.append(&Label::builder().margin_ellipsized(4).bold().build());
                            let count_box = gtk::Box::builder().spacing(4).build();
                            album_box.append(&count_box);
                            count_box.append(&Label::builder().margin_start(4).subscript().build());
                            count_box.append(&Label::builder().label("Songs").subscript().build());
                            album_row.append(&next_icon());
                            Box::new(album_row)
                        }), Box::new(|rc, list_item| {
                            let (album_string, count, _, _) = rc.borrow();
                            let album_box = list_item.child().first_child().and_downcast::<gtk::Box>().unwrap();
                            album_box.first_child().set_label(or_none(album_string));
                            album_box.last_child().first_child().set_label(&count.to_string());
                        }), true),
                    ], {
                        let media_controls = media_controls.clone();
                        let window_title = window_title.clone();
                        let scrolled_window = scrolled_window.clone();
                        move |rc| {
                            let (album_string, _, _, _) = rc.borrow();
                            Rc::new(Self::songs(album_string.clone(), artist_string.clone(), &media_controls))
                                .set(&window_title, &scrolled_window, history.clone(), &None);
                        }
                    },
                )
            ),
        }
    }
    pub fn songs(album_string: Option<String>, artist_string: Option<Rc<String>>, media_controls: &Wrapper) -> Self {
        let album_string = album_string.map(Rc::new);
        Self {
            body_type: BodyType::Songs,
            query1: album_string.clone(),
            query2: artist_string.clone(),
            title: or_none_static(album_string.clone()),
            subtitle: Rc::new(String::from("Songs")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(
                column_view(
                    get_current_album(artist_string, album_string, &mut get_connection()).into_iter()
                        .map(|(song, collection)| { Rc::new((media_controls.clone(), song, collection)) })
                        .collect::<Vec<_>>(),
                    vec![
                        (Box::new(|| { Box::new(Label::builder().bold().build()) }),
                            Box::new(|rc: Rc<(Wrapper, Song, Collection)>, list_item: &ListItem| {
                                let (wrapper, song, _) = &*rc;
                                let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection())
                                    .unwrap();
                                let label = list_item.child().and_downcast::<Label>().unwrap();
                                if let Some(track_number) = song.track_number {
                                    label.set_label(&track_number.to_string());
                                }
                                connect_accent_if_now_playing(song, current_song_id, label, wrapper);
                            }) as Box<dyn Fn(Rc<(Wrapper, Song, Collection)>, &ListItem)>, false),
                        (Box::new(|| {
                            Box::new(Label::builder().ellipsized().bold().margin_top(4).margin_bottom(4).build())
                        }), Box::new(|rc, list_item| {
                            let (wrapper, song, _) = &*rc;
                            let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection())
                                .unwrap();
                            let label = list_item.child().set_label(song.title_str());
                            connect_accent_if_now_playing(song, current_song_id, label, wrapper);
                        }), true),
                        (Box::new(|| { Box::new(Label::builder().bold_subscript().build()) }),
                            Box::new(|rc, list_item| {
                                let (wrapper, song, _) = &*rc;
                                let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection())
                                    .unwrap();
                                let label = list_item.child().set_label(&format(song.duration as u64));
                                connect_accent_if_now_playing(song, current_song_id, label, wrapper);
                            }), false),
                    ], {
                        let media_controls = media_controls.clone();
                        move |rc| {
                            let (_, song, collection) = &*rc;
                            media_controls.emit_by_name::<()>(SONG_SELECTED, &[&song.path, &collection.path]);
                        }
                    },
                )
            ),
        }
    }
}
