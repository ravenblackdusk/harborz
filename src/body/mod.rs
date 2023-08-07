use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use adw::{ApplicationWindow, WindowTitle};
use adw::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::dsl::{count_distinct, count_star, max, min};
use gtk::{Button, CenterBox, GestureClick, Grid, Image, Label, ScrolledWindow, Separator, Widget};
use gtk::Orientation::Vertical;
use crate::body::collection::add_collection_box;
use crate::common::{AdjustableScrolledWindow, ALBUM_ICON, ImagePathBuf, StyledLabelBuilder};
use crate::common::constant::ACCENT_BG;
use crate::common::util::{format, or_none_static, Plural};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist, path as song_path, year};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, join_path, WithCover};

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

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").margin_end(8).build()
}

fn accent_if_now_playing(label: &Label, row_id: i32, current_song_id: i32) {
    if row_id == current_song_id {
        label.add_css_class("accent");
    } else {
        label.remove_css_class("accent");
    }
}

fn connect_accent_if_now_playing(song_id: i32, current_song_id: Option<i32>, label: Label, wrapper: &Wrapper) {
    if let Some(current_song_id) = current_song_id {
        accent_if_now_playing(&label, song_id, current_song_id);
    }
    wrapper.connect_local(STREAM_STARTED, true, move |params| {
        accent_if_now_playing(&label, song_id, params[1].get::<i32>().unwrap());
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

const NAME: &'static str = "name";
const ALBUM: &'static str = "Album";
const SONG: &'static str = "Song";

impl Body {
    pub fn set_window_title(&self, window_title: &WindowTitle) {
        window_title.set_title(&self.title);
        window_title.set_subtitle(&self.subtitle);
    }
    pub fn from_body_table(body_type: BodyType, query1: Option<String>, query2: Option<String>,
        window_title: &WindowTitle, scrolled_window: &ScrolledWindow, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>,
        now_playing: &Wrapper, back_button: &Button, window: &ApplicationWindow) -> Self {
        let query1 = query1.map(Rc::new);
        let query2 = query2.map(Rc::new);
        match body_type {
            BodyType::Artists => {
                Body::artists(&window_title, &scrolled_window, history.clone(), &now_playing,
                    &Some(back_button.clone()))
            }
            BodyType::Albums => {
                Body::albums(query1.clone(), &window_title, &scrolled_window, history.clone(), &now_playing)
            }
            BodyType::Songs => { Body::songs(query1.clone(), query2.clone(), &now_playing) }
            BodyType::Collections => { Body::collections(&window, history) }
        }
    }
    pub fn put_to_history(self, scroll_adjustment: Option<f32>, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) {
        self.scroll_adjustment.set(scroll_adjustment);
        history.borrow_mut().push((Rc::new(self), true));
    }
    pub fn set(self: Rc<Self>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, back_button: &Option<Button>) {
        if let Some(back_button) = back_button { back_button.set_visible(true); }
        self.set_window_title(window_title);
        let mut history = history.borrow_mut();
        if let Some((body, _)) = history.last() {
            let Body { scroll_adjustment, .. } = body.deref();
            scroll_adjustment.set(scrolled_window.get_adjustment());
        }
        scrolled_window.set_child(Some((*self.widget).as_ref()));
        history.push((self, false));
    }
    pub fn collections(window: &ApplicationWindow, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) -> Self {
        Self {
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Collection")),
            body_type: BodyType::Collections,
            query1: None,
            query2: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new(add_collection_box(&window, history)),
        }
    }
    pub fn artists(window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, now_playing: &Wrapper, back_button: &Option<Button>) -> Self {
        let artists = songs.group_by(artist).select((artist, count_distinct(album), count_star()))
            .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap();
        Self {
            body_type: BodyType::Artists,
            query1: None,
            query2: None,
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(artists.len().number_plural("Artist")),
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                let artists_box = gtk::Box::builder().orientation(Vertical).build();
                for (artist_string, album_count, song_count) in artists {
                    let artist_string = artist_string.map(Rc::new);
                    let window_title = window_title.clone();
                    let scrolled_window = scrolled_window.clone();
                    let now_playing = now_playing.clone();
                    let back_button = back_button.clone();
                    let history = history.clone();
                    let artist_row = gtk::Box::builder().spacing(8).build();
                    artists_box.append(&artist_row);
                    artists_box.append(&Separator::builder().build());
                    let gesture_click = GestureClick::new();
                    gesture_click.connect_pressed({
                        let artist_row = artist_row.clone();
                        move |_, _, _, _| { artist_row.set_property(NAME, ACCENT_BG); }
                    });
                    gesture_click.connect_stopped({
                        let artist_row = artist_row.clone();
                        move |_| { artist_row.set_property(NAME, None::<String>); }
                    });
                    gesture_click.connect_released({
                        let artist_string = artist_string.clone();
                        let artist_row = artist_row.clone();
                        move |_, _, x, y| {
                            if artist_row.contains(x, y) {
                                Rc::new(Self::albums(artist_string.clone(), &window_title, &scrolled_window,
                                    history.clone(), &now_playing,
                                )).set(&window_title, &scrolled_window, history.clone(), &back_button);
                            }
                            artist_row.set_property(NAME, None::<String>);
                        }
                    });
                    artist_row.add_controller(gesture_click);
                    let artist_box = gtk::Box::builder().orientation(Vertical)
                        .margin_start(8).margin_end(4).margin_top(8).margin_bottom(8).build();
                    artist_row.append(&artist_box);
                    artist_box.append(&Label::builder().label(&*or_none_static(artist_string)).ellipsized().bold()
                        .build());
                    let count_box = gtk::Box::builder().spacing(4).build();
                    artist_box.append(&count_box);
                    let album_count_box = gtk::Box::builder().spacing(4).build();
                    count_box.append(&album_count_box);
                    album_count_box.append(&Label::builder().label(&album_count.to_string()).subscript().build());
                    album_count_box.append(&Label::builder().label(album_count.plural(ALBUM)).subscript().build());
                    let song_count_box = gtk::Box::builder().spacing(4).build();
                    count_box.append(&song_count_box);
                    song_count_box.append(&Label::builder().label(&song_count.to_string()).subscript().build());
                    song_count_box.append(&Label::builder().label(song_count.plural(SONG)).subscript().build());
                    artist_row.append(&next_icon());
                }
                artists_box
            }),
        }
    }
    pub fn albums(artist_string: Option<Rc<String>>, window_title: &WindowTitle, scrolled_window: &ScrolledWindow,
        history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>, now_playing: &Wrapper) -> Self {
        let statement = songs.inner_join(collections).group_by(album).order_by(min(year).desc())
            .select((album, count_star(), min(path), min(song_path), min(year), max(year))).into_boxed();
        let albums = if let Some(artist_string) = artist_string.clone() {
            statement.filter(artist.eq(artist_string.deref().to_owned()))
        } else {
            statement.filter(artist.is_null())
        }.get_results::<(Option<String>, i64, Option<String>, Option<String>, Option<i32>, Option<i32>)>(&mut get_connection())
            .unwrap();
        Self {
            body_type: BodyType::Albums,
            query1: artist_string.clone(),
            query2: None,
            title: or_none_static(artist_string.clone()),
            subtitle: Rc::new(albums.len().number_plural(ALBUM)),
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                let albums_box = gtk::Box::builder().orientation(Vertical).build();
                for (album_string, count, collection_path, album_song_path, min_year, max_year) in albums {
                    let album_string = album_string.map(Rc::new);
                    let window_title = window_title.clone();
                    let scrolled_window = scrolled_window.clone();
                    let now_playing = now_playing.clone();
                    let history = history.clone();
                    let album_row = gtk::Box::builder().spacing(8).build();
                    albums_box.append(&album_row);
                    albums_box.append(&Separator::builder().build());
                    album_row.append(Image::builder().pixel_size(38).margin_start(8).build().set_cover(
                        &join_path(&collection_path.unwrap(), &album_song_path.unwrap()).cover(), ALBUM_ICON)
                    );
                    let gesture_click = GestureClick::new();
                    gesture_click.connect_pressed({
                        let album_row = album_row.clone();
                        move |_, _, _, _| { album_row.set_property(NAME, ACCENT_BG); }
                    });
                    gesture_click.connect_stopped({
                        let album_row = album_row.clone();
                        move |_| { album_row.set_property(NAME, None::<String>); }
                    });
                    gesture_click.connect_released({
                        let album_string = album_string.clone();
                        let artist_string = artist_string.clone();
                        let album_row = album_row.clone();
                        move |_, _, x, y| {
                            if album_row.contains(x, y) {
                                Rc::new(Self::songs(album_string.clone(), artist_string.clone(), &now_playing))
                                    .set(&window_title, &scrolled_window, history.clone(), &None);
                            }
                            album_row.set_property(NAME, None::<String>);
                        }
                    });
                    album_row.add_controller(gesture_click);
                    let album_box = gtk::Box::builder().orientation(Vertical).margin_top(8).margin_bottom(8).build();
                    album_row.append(&album_box);
                    album_box.append(&Label::builder().label(&*or_none_static(album_string)).margin_ellipsized(4).bold()
                        .build());
                    let year_builder = Label::builder().margin_start(4).subscript();
                    let count_box = gtk::Box::builder().spacing(4).build();
                    count_box.append(&Label::builder().label(&count.to_string()).subscript().build());
                    count_box.append(&Label::builder().label(count.plural(SONG)).subscript().build());
                    let info_box = CenterBox::builder().start_widget(
                        &if let Some(min_year) = min_year {
                            year_builder.label(&if min_year == max_year.unwrap() {
                                min_year.to_string()
                            } else {
                                format!("{} to {}", min_year, max_year.unwrap())
                            })
                        } else {
                            year_builder
                        }.build()
                    ).end_widget(&count_box).build();
                    album_box.append(&info_box);
                    album_row.append(&next_icon());
                }
                albums_box
            }),
        }
    }
    pub fn songs(album_string: Option<Rc<String>>, artist_string: Option<Rc<String>>, now_playing: &Wrapper) -> Self {
        let current_album = get_current_album(artist_string.clone(), album_string.clone(), &mut get_connection());
        Self {
            body_type: BodyType::Songs,
            query1: album_string.clone(),
            query2: artist_string,
            title: or_none_static(album_string),
            subtitle: Rc::new(current_album.len().number_plural(SONG)),
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection()).unwrap();
                let grid = Grid::new();
                for (row, (song, collection)) in current_album.into_iter().enumerate() {
                    let grid_row = (2 * row) as i32;
                    let separator_row = grid_row + 1;
                    let track_number_builder = Label::builder().bold().margin_start(8).margin_end(8);
                    let track_number_label = if let Some(track_number) = song.track_number {
                        track_number_builder.label(&track_number.to_string())
                    } else {
                        track_number_builder
                    }.build();
                    grid.attach(&track_number_label, 0, grid_row, 1, 1);
                    grid.attach(&Separator::builder().build(), 0, separator_row, 1, 1);
                    let title_label = Label::builder().label(song.title_str()).ellipsized().bold()
                        .margin_start(8).margin_end(8).margin_top(8).margin_bottom(8).build();
                    grid.attach(&title_label, 1, grid_row, 1, 1);
                    grid.attach(&Separator::builder().build(), 1, separator_row, 1, 1);
                    let duration_label = Label::builder().label(&format(song.duration as u64)).bold_subscript()
                        .margin_start(8).margin_end(8).build();
                    grid.attach(&duration_label, 2, grid_row, 1, 1);
                    grid.attach(&Separator::builder().build(), 2, separator_row, 1, 1);
                    let song_path_rc = Rc::new(song.path);
                    let collection_path_rc = Rc::new(collection.path);
                    let handle_click = |label: &Label| {
                        let gesture_click = GestureClick::new();
                        gesture_click.connect_released({
                            let now_playing = now_playing.clone();
                            let song_path_rc = song_path_rc.clone();
                            let collection_path_rc = collection_path_rc.clone();
                            move |_, _, _, _| {
                                now_playing.emit_by_name::<()>(SONG_SELECTED, &[&*song_path_rc, &*collection_path_rc]);
                            }
                        });
                        label.add_controller(gesture_click);
                    };
                    handle_click(&track_number_label);
                    connect_accent_if_now_playing(song.id, current_song_id, track_number_label, now_playing);
                    handle_click(&title_label);
                    connect_accent_if_now_playing(song.id, current_song_id, title_label, now_playing);
                    handle_click(&duration_label);
                    connect_accent_if_now_playing(song.id, current_song_id, duration_label, now_playing);
                }
                grid
            }),
        }
    }
}
