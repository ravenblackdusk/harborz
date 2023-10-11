use std::rc::Rc;
use std::sync::Arc;
use adw::NavigationPage;
use adw::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::dsl::{count_distinct, count_star, min};
use gtk::{Image, Label, Separator};
use gtk::Orientation::Vertical;
use id3::TagLike;
use crate::body::{ALBUM, ARTIST, Body, BodyType, handle_render, next_icon, SONG};
use crate::body::download::albums::albums_page;
use crate::body::merge::{KEY, add_menu_merge_button, MergeState};
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

const HARBORZ: &'static str = "Harborz";

pub fn artists_page(state: Rc<State>) -> NavigationPage {
    let body = Body::new(HARBORZ, state.clone(), Some("Artists"), Vec::new(), BodyType::Artists);
    let heading = add_menu_merge_button(ARTIST, &body.menu_button, &body.popover_box);
    let render = move || {
        let artists_box = gtk::Box::builder().orientation(Vertical).build();
        let artists = songs.inner_join(collections).group_by(artist)
            .select((artist, count_distinct(album), count_star(), min(path), min(song_path)))
            .get_results::<(Option<String>, i64, i64, Option<String>, Option<String>)>(&mut get_connection()).unwrap();
        let subtitle = artists.len().number_plural(ARTIST);
        body.window_title.set_subtitle(&subtitle);
        let merge_state = MergeState::new(ARTIST, heading.clone(), Arc::new(String::from(HARBORZ)), Rc::new(subtitle),
            artists_box.clone(), &body.action_group, &body.header_bar, &body.menu_button,
            |artists| { Box::new(artist.eq_any(artists)) }, || { Box::new(artist.is_null()) },
            |tag, artist_string| { tag.set_artist(artist_string); }, |song, artist_string| {
                update(songs.filter(id.eq(song.id))).set(artist.eq(Some(artist_string))).execute(&mut get_connection())
                    .unwrap();
            },
        );
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
                let state = state.clone();
                let artist_string = artist_string.clone();
                move || {
                    state.navigation_view.push(&albums_page(vec![logo.to_str().map(|it| { Arc::new(String::from(it)) }),
                        artist_string.clone()], state.clone(), None));
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
        body.scrolled_window.set_child(Some(&merge_state.handle_pinch()));
    };
    handle_render(render, body.rerender);
    body.navigation_page
}
