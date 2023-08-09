use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use adw::prelude::*;
use adw::WindowTitle;
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::dsl::{count_distinct, count_star, max, min};
use gtk::{CenterBox, GestureClick, Grid, Image, Label, Separator, Widget};
use gtk::Orientation::Vertical;
use id3::{Tag, TagLike, Version};
use id3::v1v2::write_to_path;
use Version::Id3v24;
use crate::body::collection::add_collection_box;
use crate::body::collection::model::Collection;
use crate::body::merge::MergeState;
use crate::common::{AdjustableScrolledWindow, ALBUM_ICON, ImagePathBuf, StyledLabelBuilder};
use crate::common::state::State;
use crate::common::util::{format, or_none_static, Plural};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist, id, path as song_path, year};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, join_path, Song, WithCover, WithPath};

pub mod collection;
mod merge;

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
    back_visible: bool,
    title: Rc<String>,
    subtitle: Rc<String>,
    popover_box: gtk::Box,
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

const ARTIST: &'static str = "Artist";
const ALBUM: &'static str = "Album";
const SONG: &'static str = "Song";

fn popover_box(state: Rc<State>, merge_state: Rc<MergeState>) -> gtk::Box {
    let gtk_box = gtk::Box::builder().orientation(Vertical).build();
    gtk_box.append(&collection::button::create(state.clone()));
    gtk_box.append(&merge_state.merge_menu_button);
    gtk_box
}

impl Body {
    pub fn set_window_title(&self, window_title: &WindowTitle) {
        window_title.set_title(&self.title);
        window_title.set_subtitle(&self.subtitle);
    }
    pub fn from_body_table(body_type: BodyType, query1: Option<String>, query2: Option<String>, state: Rc<State>,
        now_playing: &Wrapper) -> Self {
        let query1 = query1.map(Rc::new);
        let query2 = query2.map(Rc::new);
        match body_type {
            BodyType::Artists => { Body::artists(state, now_playing) }
            BodyType::Albums => { Body::albums(query1.clone(), state, now_playing) }
            BodyType::Songs => { Body::songs(query1.clone(), query2.clone(), state, now_playing) }
            BodyType::Collections => { Body::collections(state) }
        }
    }
    pub fn put_to_history(self, scroll_adjustment: Option<f32>, history: Rc<RefCell<Vec<(Rc<Body>, bool)>>>) {
        self.scroll_adjustment.set(scroll_adjustment);
        history.borrow_mut().push((Rc::new(self), true));
    }
    pub fn set(self: Rc<Self>, state: Rc<State>) {
        state.back_button.set_visible(self.back_visible);
        self.set_window_title(&state.window_title);
        state.menu_button.set_visible(if self.popover_box.first_child() == None {
            false
        } else {
            state.menu_button.popover().unwrap().set_child(Some(&self.popover_box));
            true
        });
        state.scrolled_window.set_child(Some((*self.widget).as_ref()));
    }
    pub fn set_with_history(self: Rc<Self>, state: Rc<State>) {
        self.clone().set(state.clone());
        let mut history = state.history.borrow_mut();
        if let Some((body, _)) = history.last() {
            let Body { scroll_adjustment, .. } = body.deref();
            scroll_adjustment.set(state.scrolled_window.get_adjustment());
        }
        history.push((self, false));
    }
    pub fn collections(state: Rc<State>) -> Self {
        Self {
            back_visible: true,
            title: Rc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Collection")),
            popover_box: gtk::Box::builder().orientation(Vertical).build(),
            body_type: BodyType::Collections,
            query1: None,
            query2: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new(add_collection_box(state)),
        }
    }
    pub fn artists(state: Rc<State>, now_playing: &Wrapper) -> Self {
        let artists = songs.group_by(artist).select((artist, count_distinct(album), count_star()))
            .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap();
        let title = Rc::new(String::from("Harborz"));
        let subtitle = Rc::new(artists.len().number_plural(ARTIST));
        let merge_state = MergeState::new(ARTIST, state.clone(), title.clone(), subtitle.clone(),
            |artists, artist_string, has_none| {
                let in_filter = artist.eq_any(artists.iter().filter_map(|it| {
                    (!Rc::ptr_eq(&it, &artist_string)).then_some(Some(it.to_string()))
                }).collect::<Vec<_>>());
                let statement = songs.inner_join(collections).into_boxed();
                let filtered_statement = if has_none {
                    statement.filter(in_filter.or(artist.is_null()))
                } else {
                    statement.filter(in_filter)
                };
                for (song, collection) in filtered_statement
                    .get_results::<(Song, Collection)>(&mut get_connection()).unwrap() {
                    let current_path = (&song, &collection).path();
                    let mut tag = Tag::read_from_path(&current_path).unwrap();
                    tag.set_artist(artist_string.deref());
                    write_to_path(current_path, &tag, Id3v24).unwrap();
                    update(songs.filter(id.eq(song.id))).set(artist.eq(Some(artist_string.to_string())))
                        .execute(&mut get_connection()).unwrap();
                }
            },
        );
        Self {
            back_visible: false,
            title,
            subtitle,
            popover_box: popover_box(state.clone(), merge_state.clone()),
            body_type: BodyType::Artists,
            query1: None,
            query2: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                let artists_box = gtk::Box::builder().orientation(Vertical).build();
                for (artist_string, album_count, song_count) in artists {
                    let artist_string = artist_string.map(Rc::new);
                    let now_playing = now_playing.clone();
                    let artist_row = gtk::Box::builder().spacing(8).build();
                    if let Some(artist_string) = artist_string.clone() {
                        unsafe { artist_row.set_data("key", artist_string); }
                    }
                    artists_box.append(&artist_row);
                    artists_box.append(&Separator::builder().build());
                    merge_state.clone().handle_click(&artist_row, {
                        let artist_string = artist_string.clone();
                        let state = state.clone();
                        move || {
                            Rc::new(Self::albums(artist_string.clone(), state.clone(), &now_playing))
                                .set_with_history(state.clone());
                        }
                    });
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
                merge_state.handle_pinch(artists_box)
            }),
        }
    }
    pub fn albums(artist_string: Option<Rc<String>>, state: Rc<State>, now_playing: &Wrapper) -> Self {
        let statement = songs.inner_join(collections).group_by(album).order_by(min(year).desc())
            .select((album, count_star(), min(path), min(song_path), min(year), max(year))).into_boxed();
        let albums = if let Some(artist_string) = artist_string.clone() {
            statement.filter(artist.eq(artist_string.deref().to_owned()))
        } else {
            statement.filter(artist.is_null())
        }.get_results::<(Option<String>, i64, Option<String>, Option<String>, Option<i32>, Option<i32>)>(&mut get_connection())
            .unwrap();
        let title = or_none_static(artist_string.clone());
        let subtitle = Rc::new(albums.len().number_plural(ALBUM));
        let merge_state = MergeState::new(ALBUM, state.clone(), title.clone(), subtitle.clone(),
            |albums, album_string, has_none| {
                let in_filter = album.eq_any(albums.iter().filter_map(|it| {
                    (!Rc::ptr_eq(&it, &album_string)).then_some(Some(it.to_string()))
                }).collect::<Vec<_>>());
                let statement = songs.inner_join(collections).into_boxed();
                let filtered_statement = if has_none {
                    statement.filter(in_filter.or(album.is_null()))
                } else {
                    statement.filter(in_filter)
                };
                for (song, collection) in filtered_statement.get_results::<(Song, Collection)>(&mut get_connection())
                    .unwrap() {
                    let current_path = (&song, &collection).path();
                    let mut tag = Tag::read_from_path(&current_path).unwrap();
                    tag.set_album(album_string.deref());
                    write_to_path(current_path, &tag, Id3v24).unwrap();
                    update(songs.filter(id.eq(song.id))).set(album.eq(Some(album_string.to_string())))
                        .execute(&mut get_connection()).unwrap();
                }
            },
        );
        Self {
            back_visible: true,
            title,
            subtitle,
            popover_box: popover_box(state.clone(), merge_state.clone()),
            body_type: BodyType::Albums,
            query1: artist_string.clone(),
            query2: None,
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                let albums_box = gtk::Box::builder().orientation(Vertical).build();
                for (album_string, count, collection_path, album_song_path, min_year, max_year) in albums {
                    let now_playing = now_playing.clone();
                    let album_string = album_string.map(Rc::new);
                    let album_row = gtk::Box::builder().spacing(8).build();
                    if let Some(album_string) = album_string.clone() {
                        unsafe { album_row.set_data("key", album_string); }
                    }
                    albums_box.append(&album_row);
                    albums_box.append(&Separator::builder().build());
                    album_row.append(Image::builder().pixel_size(38).margin_start(8).build().set_cover(
                        &join_path(&collection_path.unwrap(), &album_song_path.unwrap()).cover(), ALBUM_ICON)
                    );
                    merge_state.clone().handle_click(&album_row, {
                        let album_string = album_string.clone();
                        let artist_string = artist_string.clone();
                        let state = state.clone();
                        move || {
                            Rc::new(Self::songs(album_string.clone(), artist_string.clone(), state.clone(),
                                &now_playing,
                            )).set_with_history(state.clone());
                        }
                    });
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
                merge_state.handle_pinch(albums_box)
            }),
        }
    }
    pub fn songs(album_string: Option<Rc<String>>, artist_string: Option<Rc<String>>, state: Rc<State>,
        now_playing: &Wrapper) -> Self {
        let current_album = get_current_album(artist_string.clone(), album_string.clone(), &mut get_connection());
        Self {
            back_visible: true,
            popover_box: {
                let gtk_box = gtk::Box::builder().orientation(Vertical).build();
                gtk_box.append(&collection::button::create(state));
                gtk_box
            },
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
