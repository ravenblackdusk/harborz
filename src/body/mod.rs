use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use adw::{ApplicationWindow, WindowTitle};
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
use crate::body::collection::add_collection_box;
use crate::body::collection::model::Collection;
use crate::common::{AdjustableScrolledWindow, BoldLabelBuilder, EllipsizedLabelBuilder, SubscriptLabelBuilder};
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
#[diesel(table_name = crate::schema::history_bodies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct HistoryBody {
    pub id: i32,
    pub query: Option<String>,
    pub body_type: BodyType,
    pub scroll_adjustment: Option<f32>,
}

#[derive(Debug, PartialEq, diesel_derive_enum::DbEnum)]
pub enum BodyType {
    Artists,
    Albums,
    Songs,
    Collections,
}

pub struct Body {
    pub title: Rc<String>,
    pub subtitle: Rc<String>,
    pub body_type: BodyType,
    pub query: Option<Rc<String>>,
    pub scroll_adjustment: Cell<Option<f32>>,
    pub widget: Box<dyn AsRef<Widget>>,
}

fn column_view<T: 'static, S: Fn(Rc<T>, &ListItem) + ?Sized + 'static, F: Fn(Rc<T>) + 'static>(row_items: Vec<Rc<T>>,
    columns: Vec<(Box<S>, bool)>, on_row_activated: F) -> ColumnView {
    let store = ListStore::new(BoxedAnyObject::static_type());
    for row_item in row_items.iter() {
        store.append(&BoxedAnyObject::new(row_item.clone()));
    }
    let column_view = ColumnView::builder().single_click_activate(true).model(&NoSelection::new(Some(store)))
        .show_row_separators(true).build();
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
    column_view
}

fn or_none_label(label: &Option<String>) -> Label {
    Label::builder().label(or_none(label)).margin_ellipsized(4).bold().build()
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").build()
}

fn bold_if_now_playing(label: &Label, row_id: i32, current_song_id: i32) {
    let attr_list = label.attributes().unwrap_or_else(AttrList::new);
    attr_list.change(AttrInt::new_weight(if row_id == current_song_id {
        label.add_css_class("accent");
        Weight::Bold
    } else {
        label.remove_css_class("accent");
        Weight::Normal
    }));
    label.set_attributes(Some(&attr_list));
}

fn connect_bold_if_now_playing(song: &Song, current_song_id: Option<i32>, list_item: &ListItem, label: Label,
    wrapper: &Wrapper) {
    let id = song.id;
    if let Some(current_song_id) = current_song_id {
        bold_if_now_playing(&label, id, current_song_id);
    }
    list_item.set_child(Some(&label));
    wrapper.connect_local(STREAM_STARTED, true, move |params| {
        bold_if_now_playing(&label, id, params[1].get::<i32>().unwrap());
        None
    });
}

impl Body {
    pub fn put_to_history(self, scroll_adjustment: Option<f32>, history: Rc<RefCell<Vec<(Body, bool)>>>) {
        self.scroll_adjustment.set(scroll_adjustment);
        history.borrow_mut().push((self, true));
    }
    pub fn set(self, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Self, bool)>>>, adjust_scroll: bool) {
        window_title.set_title(self.title.as_str());
        window_title.set_subtitle(self.subtitle.as_str());
        let mut history = history.borrow_mut();
        if let Some((Body { scroll_adjustment, .. }, _)) = history.last() {
            scroll_adjustment.set(scrolled_window.get_adjustment());
        }
        scrolled_window.set_child(Some((*self.widget).as_ref()));
        history.push((self, adjust_scroll));
    }
    pub fn collections(window: &ApplicationWindow) -> Self {
        Self {
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Collection")),
            body_type: BodyType::Collections,
            query: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new(add_collection_box(&window)),
        }
    }
    pub fn artists(window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Self, bool)>>>, media_controls: &Wrapper) -> Self {
        Self {
            body_type: BodyType::Artists,
            query: None,
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Artists")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(
                column_view(
                    songs.group_by(artist).select((artist, count_distinct(album), count_star()))
                        .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap().into_iter()
                        .map(Rc::new).collect::<Vec<_>>(),
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
                        let window_title = window_title.clone();
                        let scrolled_window = scrolled_window.clone();
                        let media_controls = media_controls.clone();
                        move |rc| {
                            let (artist_string, _, _) = rc.borrow();
                            Self::albums(artist_string.clone(), &window_title, &scrolled_window, history.clone(), &media_controls)
                                .set(&window_title, &scrolled_window, history.clone(), false);
                        }
                    },
                )
            ),
        }
    }
    pub fn albums(artist_string: Option<String>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Self, bool)>>>, media_controls: &Wrapper) -> Self {
        let artist_string = artist_string.map(Rc::new);
        Self {
            body_type: BodyType::Albums,
            query: artist_string.clone(),
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
                        (Box::new(|rc: Rc<(Option<String>, i64, Option<String>, Option<String>)>, list_item: &ListItem| {
                            let (_, _, collection_path, album_song_path) = rc.borrow();
                            let cover = join_path(&collection_path.clone().unwrap(), &album_song_path.clone().unwrap()).cover();
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
                            count_box.append(&Label::builder().label(count.to_string()).margin_start(4).subscript().build());
                            count_box.append(&Label::builder().label("Songs").subscript().build());
                            album_box.append(&count_box);
                            album_row.append(&album_box);
                            album_row.append(&next_icon());
                            list_item.set_child(Some(&album_row));
                        }), true),
                    ], {
                        let media_controls = media_controls.clone();
                        let window_title = window_title.clone();
                        let scrolled_window = scrolled_window.clone();
                        move |rc| {
                            let (album_string, _, _, _) = rc.borrow();
                            Self::songs(album_string.clone(), artist_string.clone(), &media_controls)
                                .set(&window_title, &scrolled_window, history.clone(), false);
                        }
                    },
                )
            ),
        }
    }
    pub fn songs(album_string: Option<String>, artist_string: Option<Rc<String>>, media_controls: &Wrapper) -> Self {
        let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection()).unwrap();
        let album_string = album_string.map(Rc::new);
        Self {
            body_type: BodyType::Songs,
            query: album_string.clone(),
            title: or_none_static(album_string.clone()),
            subtitle: Rc::new(String::from("Songs")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(
                column_view(
                    get_current_album(artist_string, album_string, &mut get_connection()).into_iter()
                        .map(|(song, collection)| { Rc::new((media_controls.clone(), song, collection, current_song_id)) })
                        .collect::<Vec<_>>(),
                    vec![
                        (Box::new(|rc: Rc<(Wrapper, Song, Collection, Option<i32>)>, list_item: &ListItem| {
                            let (wrapper, song, _, current_song_id) = &*rc;
                            let label = Label::new(song.track_number.map(|it| { it.to_string() }).as_deref());
                            connect_bold_if_now_playing(song, *current_song_id, list_item, label, wrapper);
                        }) as Box<dyn Fn(Rc<(Wrapper, Song, Collection, Option<i32>)>, &ListItem)>, false),
                        (Box::new(|rc, list_item| {
                            let (wrapper, song, _, current_song_id) = &*rc;
                            let label = Label::builder().label(song.title_str()).ellipsized().margin_top(4).margin_bottom(4).build();
                            connect_bold_if_now_playing(song, *current_song_id, list_item, label, wrapper);
                        }), true),
                        (Box::new(|rc, list_item| {
                            let (wrapper, song, _, current_song_id) = &*rc;
                            let label = Label::builder().label(&format(song.duration as u64)).subscript().build();
                            connect_bold_if_now_playing(song, *current_song_id, list_item, label, wrapper);
                        }), false),
                    ], {
                        let media_controls = media_controls.clone();
                        move |rc| {
                            let (_, song, collection, _) = &*rc;
                            media_controls.emit_by_name::<()>(SONG_SELECTED, &[&song.path, &collection.path]);
                        }
                    },
                )
            ),
        }
    }
}
