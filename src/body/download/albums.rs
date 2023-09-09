use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use adw::prelude::*;
use bytes::Bytes;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, update};
use diesel::dsl::{count_star, max, min};
use gtk::{Image, Label, Separator};
use gtk::Align::Center;
use gtk::Orientation::Vertical;
use id3::TagLike;
use metadata_fetch::{ArtistSearch, DownloadArtistEvent::*, MetadataFetcher};
use crate::body::{ALBUM, Body, BodyType, next_icon, popover_box, SONG};
use crate::body::download::{append_download_button, METAL_ARCHIVES, save};
use crate::body::download::songs::songs_body;
use crate::body::merge::{KEY, MergeState};
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

pub fn albums(mut params: Vec<Option<Arc<String>>>, state: Rc<State>) -> Body {
    let statement = songs.inner_join(collections).group_by(album).order_by(min(year).desc())
        .select((album, count_star(), min(path), min(song_path), min(year), max(year))).into_boxed();
    let artist_string = params.pop().unwrap();
    let logo_or_photo = params.pop().unwrap();
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
    Body {
        back_visible: true,
        title,
        subtitle,
        popover_box: {
            let gtk_box = popover_box(state.clone(), merge_state.clone());
            let logo_or_photo = logo_or_photo.clone().unwrap();
            if let Some(artist_string) = artist_string.clone() {
                append_download_button("logo & photo", &gtk_box, state.clone(), {
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
                }, move |images_vec, i| {
                    save_option(logo_or_photo.sibling_logo(), images_vec[0].borrow_mut().remove(i));
                    save_option(logo_or_photo.sibling_photo(), images_vec[1].borrow_mut().remove(i));
                });
            }
            gtk_box
        },
        body_type: BodyType::Albums,
        params: vec![logo_or_photo, artist_string.clone()],
        scroll_adjustment: Cell::new(None),
        widget: Box::new({
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
                        Rc::new(songs_body(
                            vec![cover.to_str().map(|it| { Arc::new(String::from(it)) }), artist_string.clone(),
                                album_string.clone()], state.clone(),
                        )).set_with_history(state.clone());
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
            merge_state.handle_pinch()
        }),
    }
}
