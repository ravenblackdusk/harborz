use std::cell::{Cell, RefCell};
use std::fs::hard_link;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use adw::gio::{Cancellable, ListStore};
use adw::prelude::*;
use adw::WindowTitle;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::dsl::{count_distinct, count_star, max, min};
use gtk::{Button, CenterBox, FileDialog, FileFilter, GestureClick, Grid, Image, Label, Separator, Widget};
use gtk::Orientation::Vertical;
use id3::TagLike;
use log::{error, warn};
use crate::body::collection::add_collection_box;
use crate::body::merge::{KEY, MergeState};
use crate::common::{AdjustableScrolledWindow, ALBUM_ICON, ImagePathBuf, StyledLabelBuilder};
use crate::common::constant::INSENSITIVE_FG;
use crate::common::state::State;
use crate::common::util::{format, or_none_arc, Plural};
use crate::common::wrapper::{SONG_SELECTED, STREAM_STARTED, Wrapper};
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::config::dsl::config;
use crate::schema::songs::{album, artist, id, path as song_path, year};
use crate::schema::songs::dsl::songs;
use crate::song::{get_current_album, join_path, WithCover};

pub mod collection;
mod merge;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::bodies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BodyTable {
    pub id: i32,
    pub body_type: BodyType,
    pub scroll_adjustment: Option<f32>,
    pub navigation_type: NavigationType,
    pub params: String,
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
    title: Arc<String>,
    subtitle: Rc<String>,
    popover_box: gtk::Box,
    pub body_type: BodyType,
    pub params: Vec<Option<Arc<String>>>,
    pub scroll_adjustment: Cell<Option<f32>>,
    pub widget: Box<dyn AsRef<Widget>>,
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").margin_start(2).margin_end(8).build()
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
    pub fn from_body_table(body_type: BodyType, params: Vec<Option<String>>, state: Rc<State>, now_playing: &Wrapper)
        -> Self {
        let params = params.into_iter().map(|it| { it.map(Arc::new) }).collect();
        match body_type {
            BodyType::Artists => { Body::artists(state, now_playing) }
            BodyType::Albums => { Body::albums(params, state, now_playing) }
            BodyType::Songs => { Body::songs(params, state, now_playing) }
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
            title: Arc::new(String::from("Harborz")),
            subtitle: Rc::new(String::from("Collection")),
            popover_box: gtk::Box::builder().orientation(Vertical).build(),
            body_type: BodyType::Collections,
            params: Vec::new(),
            scroll_adjustment: Cell::new(None),
            widget: Box::new(add_collection_box(state)),
        }
    }
    pub fn artists(state: Rc<State>, now_playing: &Wrapper) -> Self {
        let artists = songs.group_by(artist).select((artist, count_distinct(album), count_star()))
            .get_results::<(Option<String>, i64, i64)>(&mut get_connection()).unwrap();
        let title = Arc::new(String::from("Harborz"));
        let subtitle = Rc::new(artists.len().number_plural(ARTIST));
        let artists_box = gtk::Box::builder().orientation(Vertical).build();
        let merge_state = MergeState::new(ARTIST, state.clone(), title.clone(), subtitle.clone(), artists_box.clone(),
            |artists| { Box::new(artist.eq_any(artists)) }, || { Box::new(artist.is_null()) },
            |tag, artist_string| { tag.set_artist(artist_string); }, |song, artist_string| {
                update(songs.filter(id.eq(song.id))).set(artist.eq(Some(artist_string))).execute(&mut get_connection())
                    .unwrap();
            },
        );
        Self {
            back_visible: false,
            title,
            subtitle,
            popover_box: popover_box(state.clone(), merge_state.clone()),
            body_type: BodyType::Artists,
            params: Vec::new(),
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                for (artist_string, album_count, song_count) in artists {
                    let artist_string = artist_string.map(Arc::new);
                    let now_playing = now_playing.clone();
                    let artist_row = gtk::Box::builder().spacing(8).build();
                    if let Some(artist_string) = artist_string.clone() {
                        unsafe { artist_row.set_data(KEY, artist_string); }
                    }
                    artists_box.append(&artist_row);
                    artists_box.append(&Separator::builder().build());
                    merge_state.clone().handle_click(&artist_row, {
                        let artist_string = artist_string.clone();
                        let state = state.clone();
                        move || {
                            Rc::new(Self::albums(vec![artist_string.clone()], state.clone(), &now_playing))
                                .set_with_history(state.clone());
                        }
                    });
                    let artist_box = gtk::Box::builder().orientation(Vertical)
                        .margin_start(8).margin_end(4).margin_top(8).margin_bottom(8).build();
                    artist_row.append(&artist_box);
                    artist_box.append(&Label::builder().label(&*or_none_arc(artist_string)).ellipsized().build());
                    let count_box = gtk::Box::builder().spacing(4).name(INSENSITIVE_FG).build();
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
                merge_state.handle_pinch()
            }),
        }
    }
    pub fn albums(mut params: Vec<Option<Arc<String>>>, state: Rc<State>, now_playing: &Wrapper) -> Self {
        let statement = songs.inner_join(collections).group_by(album).order_by(min(year).desc())
            .select((album, count_star(), min(path), min(song_path), min(year), max(year))).into_boxed();
        let artist_string = params.pop().unwrap();
        let albums = if let Some(artist_string) = &artist_string {
            statement.filter(artist.eq(artist_string.as_ref()))
        } else {
            statement.filter(artist.is_null())
        }.get_results::<(Option<String>, i64, Option<String>, Option<String>, Option<i32>, Option<i32>)>(&mut get_connection())
            .unwrap();
        let title = or_none_arc(artist_string.clone());
        let subtitle = Rc::new(albums.len().number_plural(ALBUM));
        let albums_box = gtk::Box::builder().orientation(Vertical).build();
        let merge_state = MergeState::new(ALBUM, state.clone(), title.clone(), subtitle.clone(), albums_box.clone(),
            |albums| { Box::new(album.eq_any(albums)) }, || { Box::new(album.is_null()) },
            |tag, album_string| { tag.set_album(album_string); }, |song, album_string| {
                update(songs.filter(id.eq(song.id))).set(album.eq(Some(album_string))).execute(&mut get_connection())
                    .unwrap();
            },
        );
        Self {
            back_visible: true,
            title,
            subtitle,
            popover_box: popover_box(state.clone(), merge_state.clone()),
            body_type: BodyType::Albums,
            params: vec![artist_string.clone()],
            scroll_adjustment: Cell::new(None),
            widget: Box::new({
                for (album_string, count, collection_path, album_song_path, min_year, max_year) in albums {
                    let now_playing = now_playing.clone();
                    let album_string = album_string.map(Arc::new);
                    let album_row = gtk::Box::builder().spacing(8).build();
                    if let Some(album_string) = album_string.clone() {
                        unsafe { album_row.set_data(KEY, album_string); }
                    }
                    albums_box.append(&album_row);
                    albums_box.append(&Separator::builder().build());
                    let cover = join_path(&collection_path.unwrap(), &album_song_path.unwrap()).cover();
                    album_row.append(Image::builder().pixel_size(46).margin_start(8).build()
                        .set_cover(&cover, ALBUM_ICON));
                    merge_state.clone().handle_click(&album_row, {
                        let album_string = album_string.clone();
                        let artist_string = artist_string.clone();
                        let state = state.clone();
                        move || {
                            Rc::new(Self::songs(
                                vec![cover.to_str().map(|it| { Arc::new(String::from(it)) }), artist_string.clone(),
                                    album_string.clone()], state.clone(), &now_playing,
                            )).set_with_history(state.clone());
                        }
                    });
                    let album_box = gtk::Box::builder().orientation(Vertical).margin_top(12).margin_bottom(12).build();
                    album_row.append(&album_box);
                    album_box.append(&Label::builder().label(&*or_none_arc(album_string)).margin_ellipsized(4)
                        .build());
                    let year_builder = Label::builder().margin_start(4).subscript();
                    let count_box = gtk::Box::builder().spacing(4).build();
                    count_box.append(&Label::builder().label(&count.to_string()).subscript().build());
                    count_box.append(&Label::builder().label(count.plural(SONG)).subscript().build());
                    let info_box = CenterBox::builder().name(INSENSITIVE_FG).start_widget(
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
                merge_state.handle_pinch()
            }),
        }
    }
    pub fn songs(mut params: Vec<Option<Arc<String>>>, state: Rc<State>, now_playing: &Wrapper) -> Self {
        let album_string = params.pop().unwrap();
        let artist_string = params.pop().unwrap();
        let cover = params.pop().unwrap();
        let current_album = get_current_album(artist_string.clone(), album_string.clone(), &mut get_connection());
        Self {
            back_visible: true,
            popover_box: {
                let gtk_box = gtk::Box::builder().orientation(Vertical).build();
                gtk_box.append(&collection::button::create(state.clone()));
                if let Some(cover) = cover.clone() {
                    let select_cover = Button::builder().label("Album cover")
                        .tooltip_text("Choose Album cover from local files").build();
                    gtk_box.append(&select_cover);
                    select_cover.connect_clicked({
                        let state = state.clone();
                        move |_| {
                            let file_filter = FileFilter::new();
                            file_filter.add_pixbuf_formats();
                            let list_store = ListStore::new::<FileFilter>();
                            list_store.append(&file_filter);
                            FileDialog::builder().title("Album cover").accept_label("Choose").filters(&list_store)
                                .build().open(Some(&state.window), Cancellable::NONE, {
                                let cover = cover.clone();
                                move |file| {
                                    match file {
                                        Ok(file) => {
                                            if let Err(error) = hard_link(file.path().unwrap(), cover.as_ref()) {
                                                error!("error creating hard link from [{}] to [{:?}] [{}]", file, cover,
                                                    error);
                                            }
                                        }
                                        Err(error) => { warn!("error choosing file [{}]", error); }
                                    }
                                }
                            });
                            state.menu_button.popdown();
                        }
                    });
                }
                gtk_box
            },
            body_type: BodyType::Songs,
            params: vec![cover, artist_string, album_string.clone()],
            title: or_none_arc(album_string),
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
                        .margin_start(8).margin_end(8).margin_top(12).margin_bottom(12).build();
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
