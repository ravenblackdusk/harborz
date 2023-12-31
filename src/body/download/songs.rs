use std::cell::Cell;
use std::collections::HashMap;
use std::fs::hard_link;
use std::rc::Rc;
use std::sync::Arc;
use adw::NavigationPage;
use adw::gdk::pango::{AttrInt, AttrList};
use adw::gdk::pango::AttrType::Weight;
use adw::gdk::pango::Weight::Bold;
use adw::gio::{Cancellable, ListStore};
use adw::prelude::*;
use diesel::RunQueryDsl;
use gtk::{Button, FileDialog, FileFilter, GestureClick, Grid, Label, Separator};
use gtk::Orientation::Vertical;
use log::{error, warn};
use metadata_fetch::{AlbumSearch, MetadataFetcher};
use metadata_fetch::DownloadAlbumEvent::{Cover, SearchResult};
use crate::body::{action_name, Body, BodyType, handle_render, POP_DOWN, SONG};
use crate::body::download::{append_download_button, handle_scroll, METAL_ARCHIVES, save};
use crate::common::constant::INSENSITIVE_FG;
use crate::common::state::State;
use crate::common::StyledLabelBuilder;
use crate::common::util::{format, or_none_arc, Plural};
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::config::dsl::config;
use crate::song::{get_current_album, join_path};

pub fn songs_page(params: Vec<Option<Arc<String>>>, state: Rc<State>, scroll_adjustment: Option<f64>)
    -> NavigationPage {
    let (album_string, artist_string, cover) = {
        let mut params = params.clone();
        (params.pop().unwrap(), params.pop().unwrap(), params.pop().unwrap())
    };
    let body = Body::new(&*or_none_arc(album_string.clone()), state.clone(), None, params, BodyType::Songs);
    let cover = cover.clone().unwrap();
    let select_cover = Button::builder().label("Choose cover").tooltip_text("Choose album cover from local files")
        .build();
    body.popover_box.append(&select_cover);
    select_cover.connect_clicked({
        let state = state.clone();
        let cover = cover.clone();
        move |select_cover| {
            let file_filter = FileFilter::new();
            file_filter.add_pixbuf_formats();
            let list_store = ListStore::new::<FileFilter>();
            list_store.append(&file_filter);
            FileDialog::builder().title("Album cover").accept_label("Choose").filters(&list_store).build()
                .open(Some(&state.window), Cancellable::NONE, {
                    let cover = cover.clone();
                    move |file| {
                        match file {
                            Ok(file) => {
                                if let Err(error) = hard_link(file.path().unwrap(), cover.as_ref()) {
                                    error!("error creating hard link from [{file}] to [{cover}] [{error}]");
                                }
                            }
                            Err(error) => { warn!("error choosing file [{error}]"); }
                        }
                    }
                });
            select_cover.activate_action(&action_name(POP_DOWN), None).unwrap();
        }
    });
    if let Some(artist_string) = artist_string.clone() {
        if let Some(album_string) = album_string.clone() {
            append_download_button("cover", &body.popover_box, {
                let artist_string = artist_string.clone();
                let album_string = album_string.clone();
                move |sender| { METAL_ARCHIVES.download_cover(&artist_string, &album_string, sender); }
            }, 1, move |search_result, handle_search_result, handle_bytes| {
                match search_result {
                    SearchResult(search_result) => { handle_search_result(search_result); }
                    Cover(i, cover) => {
                        handle_bytes(i, cover, Box::new(|gtk_box, image| { gtk_box.prepend(image); }), 0);
                    }
                }
            }, |AlbumSearch { artist, album, album_type }| {
                let gtk_box = gtk::Box::builder().orientation(Vertical).spacing(4).hexpand(true).margin_start(4)
                    .build();
                gtk_box.append(&Label::builder().label(&album).bold().wrap(true).build());
                gtk_box.append(&Label::builder().label(&album_type).wrap(true).build());
                gtk_box.append(&Label::builder().label(&artist).wrap(true).subscript().name(INSENSITIVE_FG)
                    .build());
                gtk_box
            }, move |images_vec, i| {
                save(&*cover, images_vec[0].borrow_mut().remove(i).unwrap());
            }, body.menu_button.clone());
        }
    }
    let adjustment = body.scrolled_window.vadjustment();
    let render = move || {
        let current_album = get_current_album(&artist_string, &album_string, &mut get_connection());
        body.window_title.set_subtitle(&current_album.len().number_plural(SONG));
        let Config { current_song_id, .. } = config.get_result::<Config>(&mut get_connection()).unwrap();
        let current_song_id = Cell::new(current_song_id);
        let grid = Grid::new();
        let song_id_to_labels = current_album.iter().enumerate().map(|(row, (song, collection))| {
            let grid_row = (2 * row) as i32;
            let separator_row = grid_row + 1;
            let track_number_builder = Label::builder().margin_start(8).margin_end(8);
            let track_number_label = if let Some(track_number) = song.track_number {
                track_number_builder.label(&track_number.to_string())
            } else {
                track_number_builder
            }.build();
            grid.attach(&track_number_label, 0, grid_row, 1, 1);
            grid.attach(&Separator::builder().build(), 0, separator_row, 1, 1);
            let title_label = Label::builder().label(song.title_str()).ellipsized()
                .margin_start(8).margin_end(8).margin_top(12).margin_bottom(12).build();
            grid.attach(&title_label, 1, grid_row, 1, 1);
            grid.attach(&Separator::builder().build(), 1, separator_row, 1, 1);
            let duration_label = Label::builder().label(&format(song.duration as u64)).subscript()
                .margin_start(8).margin_end(8).build();
            grid.attach(&duration_label, 2, grid_row, 1, 1);
            grid.attach(&Separator::builder().build(), 2, separator_row, 1, 1);
            let labels = vec![track_number_label, title_label, duration_label];
            let path = Rc::new(join_path(&collection.path, &song.path));
            for label in &labels {
                let gesture_click = GestureClick::new();
                gesture_click.connect_released({
                    let state = state.clone();
                    let path = path.clone();
                    move |_, _, _, _| { state.window_actions.song_selected.activate(path.to_str().unwrap()); }
                });
                label.add_controller(gesture_click);
            }
            (song.id, labels)
        }).collect::<HashMap<_, _>>();
        state.window_actions.stream_started.action.connect_activate(move |_, params| {
            let started_song_id = params.unwrap().get::<i32>().unwrap();
            if let Some(labels) = song_id_to_labels.get(&started_song_id) {
                for label in labels {
                    label.add_css_class("accent");
                    let attr_list = label.attributes().unwrap_or_else(AttrList::new);
                    attr_list.insert(AttrInt::new_weight(Bold));
                    label.set_attributes(Some(&attr_list));
                }
                if let Some(current_id) = current_song_id.get() {
                    if current_id != started_song_id {
                        current_song_id.set(Some(started_song_id));
                        if let Some(labels) = song_id_to_labels.get(&current_id) {
                            for label in labels {
                                label.remove_css_class("accent");
                                label.set_attributes(label.attributes().unwrap_or_else(AttrList::new)
                                    .filter(|it| { !(it.type_() == Weight) }).as_ref());
                            }
                        }
                    }
                }
            }
        });
        body.scrolled_window.set_child(Some(&grid));
    };
    handle_scroll(scroll_adjustment, adjustment);
    handle_render(render, body.rerender);
    body.navigation_page
}
