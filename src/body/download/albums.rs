use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use adw::NavigationPage;
use adw::prelude::*;
use bytes::Bytes;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::dsl::{count_star, max, min};
use gtk::{Image, Label, Separator};
use gtk::Align::Center;
use gtk::Orientation::Vertical;
use id3::TagLike;
use metadata_fetch::{ArtistSearch, DownloadArtistEvent::*, MetadataFetcher};
use crate::body::{ALBUM, Body, BodyType, handle_render, next_icon, SONG};
use crate::body::download::{append_download_button, handle_scroll, METAL_ARCHIVES, save};
use crate::body::download::songs::songs_page;
use crate::body::merge::{KEY, add_menu_merge_button, MergeState};
use crate::common::{FOLDER_MUSIC_ICON, ImagePathBuf, StyledLabelBuilder};
use crate::common::constant::INSENSITIVE_FG;
use crate::common::state::State;
use crate::common::util::{or_none_arc, Plural};
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::collections::path;
use crate::schema::songs::{album, artist, id, path as song_path, year};
use crate::schema::songs::dsl::songs;
use crate::song::{join_path, WithImage};

fn save_option(image_path: impl AsRef<Path>, bytes: Option<Bytes>) {
    if let Some(bytes) = bytes {
        save(image_path, bytes);
    }
}

fn image_box(gtk_box: &gtk::Box) -> gtk::Box {
    gtk_box.first_child().and_downcast::<gtk::Box>().unwrap()
}

pub fn albums_page(params: Vec<Option<Arc<String>>>, state: Rc<State>, scroll_adjustment: Option<f64>)
    -> NavigationPage {
    let (artist_string, logo_or_photo) = {
        let mut params = params.clone();
        (params.pop().unwrap(), params.pop().unwrap())
    };
    let title = or_none_arc(artist_string.clone());
    let body = Body::new(&*title, state.clone(), None, params, BodyType::Albums);
    let heading = add_menu_merge_button(ALBUM, &body.menu_button, &body.popover_box);
    if let Some(artist_string) = artist_string.clone() {
        append_download_button("logo & photo", &body.popover_box, {
            let artist_string = artist_string.clone();
            move |sender| { METAL_ARCHIVES.download_artist_logo_and_photo(&artist_string, sender); }
        }, 2, move |search_result, handle_search_result, handle_bytes| {
            match search_result {
                SearchResult(search_result) => { handle_search_result(search_result); }
                Logo(i, logo) => {
                    handle_bytes(i, logo, Box::new(|gtk_box, image| { image_box(gtk_box).prepend(image); }), 0);
                }
                Photo(i, photo) => {
                    handle_bytes(i, photo, Box::new(|gtk_box, image| { image_box(gtk_box).append(image); }), 1);
                }
            }
        }, |ArtistSearch { name, genre, location }| {
            let gtk_box = gtk::Box::builder().orientation(Vertical).spacing(4).hexpand(true).margin_start(4)
                .build();
            let image_box = gtk::Box::builder().spacing(4).halign(Center).build();
            gtk_box.append(&image_box);
            gtk_box.append(&Label::builder().label(&name).bold().wrap(true).build());
            gtk_box.append(&Label::builder().label(&genre).wrap(true).build());
            gtk_box.append(&Label::builder().label(&location).wrap(true).subscript().name(INSENSITIVE_FG)
                .build());
            gtk_box
        }, {
            let logo_or_photo = logo_or_photo.clone().unwrap();
            move |images_vec, i| {
                save_option(logo_or_photo.sibling_logo(), images_vec[0].borrow_mut().remove(i));
                save_option(logo_or_photo.sibling_photo(), images_vec[1].borrow_mut().remove(i));
            }
        }, body.menu_button.clone());
    }
    let adjustment = body.scrolled_window.vadjustment();
    let render = move || {
        let statement = songs.inner_join(collections).group_by(album).order_by(min(year).desc())
            .select((album, count_star(), min(path), min(song_path), min(year), max(year))).into_boxed();
        let albums = if let Some(artist_string) = &artist_string {
            statement.filter(artist.eq(artist_string.as_ref()))
        } else {
            statement.filter(artist.is_null())
        }.get_results::<(Option<String>, i64, Option<String>, Option<String>, Option<i32>, Option<i32>)>(&mut get_connection())
            .unwrap();
        let subtitle = albums.len().number_plural(ALBUM);
        body.window_title.set_subtitle(&subtitle);
        let albums_box = gtk::Box::builder().orientation(Vertical).build();
        let merge_state = MergeState::new(ALBUM, heading.clone(), title.clone(), Rc::new(subtitle),
            albums_box.clone(), &body.action_group, &body.header_bar, &body.menu_button,
            |albums| { Box::new(album.eq_any(albums)) }, || { Box::new(album.is_null()) },
            |tag, album_string| { tag.set_album(album_string); }, |song, album_string| {
                update(songs.filter(id.eq(song.id))).set(album.eq(Some(album_string))).execute(&mut get_connection())
                    .unwrap();
            },
        );
        for (album_string, count, collection_path, album_song_path, min_year, max_year) in albums {
            let album_string = album_string.map(Arc::new);
            let album_row = gtk::Box::builder().spacing(8).build();
            if let Some(album_string) = album_string.clone() {
                unsafe { album_row.set_data(KEY, album_string); }
            }
            albums_box.append(&album_row);
            albums_box.append(&Separator::builder().build());
            let cover = join_path(&collection_path.unwrap(), &album_song_path.unwrap()).cover();
            album_row.append(Image::builder().pixel_size(46).margin_start(8).build()
                .set_or_default(&cover, FOLDER_MUSIC_ICON));
            merge_state.clone().handle_click(&album_row, {
                let album_string = album_string.clone();
                let artist_string = artist_string.clone();
                let state = state.clone();
                move || {
                    state.navigation_view.push(&songs_page(vec![cover.to_str().map(|it| { Arc::new(it.to_owned()) }),
                        artist_string.clone(), album_string.clone()], state.clone(), None));
                }
            });
            let album_box = gtk::Box::builder().orientation(Vertical)
                .margin_start(8).margin_end(4).margin_top(12).margin_bottom(12).build();
            album_row.append(&album_box);
            album_box.append(&Label::builder().label(&*or_none_arc(album_string)).ellipsized().build());
            let year_builder = Label::builder().name(INSENSITIVE_FG).ellipsized().subscript();
            let count_box = gtk::Box::builder().spacing(4).name(INSENSITIVE_FG).build();
            count_box.append(&Label::builder().label(&count.to_string()).subscript().build());
            count_box.append(&Label::builder().label(count.plural(SONG)).subscript().build());
            let info_box = if let Some(min_year) = min_year {
                year_builder.label(&if min_year == max_year.unwrap() {
                    min_year.to_string()
                } else {
                    format!("{min_year} to {}", max_year.unwrap())
                })
            } else {
                year_builder
            }.build();
            album_box.append(&info_box);
            album_row.append(&count_box);
            album_row.append(&next_icon());
        }
        body.scrolled_window.set_child(Some(&merge_state.clone().handle_pinch()));
    };
    handle_scroll(scroll_adjustment, adjustment);
    handle_render(render, body.rerender);
    body.navigation_page
}
