use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use adw::prelude::*;
use diesel::dsl::{count_distinct, count_star, min};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use gtk::Orientation::Vertical;
use gtk::{Image, Label, Separator};
use id3::TagLike;
use crate::body::{ALBUM, ARTIST, Body, BodyType, next_icon, popover_box, SONG};
use crate::body::download::albums::albums;
use crate::body::merge::{KEY, MergeState};
use crate::common::{FOLDER_MUSIC_ICON, ImagePathBuf, StyledLabelBuilder};
use crate::common::constant::INSENSITIVE_FG;
use crate::common::state::State;
use crate::common::util::{or_none_arc, Plural};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::songs::{album, artist, id, path as song_path};
use crate::schema::songs::dsl::songs;
use crate::song::{join_path, WithImage};

pub fn artists(state: Rc<State>) -> Body {
    let artists = songs.inner_join(collections).group_by(artist)
        .select((artist, count_distinct(album), count_star(), min(path), min(song_path)))
        .get_results::<(Option<String>, i64, i64, Option<String>, Option<String>)>(&mut get_connection()).unwrap();
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
    Body {
        back_visible: false,
        title,
        subtitle,
        popover_box: popover_box(state.clone(), merge_state.clone()),
        body_type: BodyType::Artists,
        params: Vec::new(),
        scroll_adjustment: Cell::new(None),
        widget: Box::new({
            for (artist_string, album_count, song_count, collection_path, artist_song_path) in artists {
                let artist_string = artist_string.map(Arc::new);
                let artist_row = gtk::Box::builder().spacing(8).build();
                if let Some(artist_string) = artist_string.clone() {
                    unsafe { artist_row.set_data(KEY, artist_string); }
                }
                artists_box.append(&artist_row);
                artists_box.append(&Separator::builder().build());
                let logo = join_path(&collection_path.unwrap(), &artist_song_path.unwrap()).logo();
                artist_row.append(Image::builder().pixel_size(46).margin_start(8).build()
                    .set_or_default(&logo, FOLDER_MUSIC_ICON));
                merge_state.clone().handle_click(&artist_row, {
                    let artist_string = artist_string.clone();
                    let state = state.clone();
                    move || {
                        Rc::new(albums(vec![logo.to_str().map(|it| { Arc::new(String::from(it)) }),
                            artist_string.clone()], state.clone())).set_with_history(state.clone());
                    }
                });
                let artist_box = gtk::Box::builder().orientation(Vertical)
                    .margin_start(8).margin_end(4).margin_top(12).margin_bottom(12).build();
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
