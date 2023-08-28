use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{channel, TryRecvError::*};
use std::thread;
use std::time::Duration;
use adw::glib::{ControlFlow::*, timeout_add_local};
use adw::MessageDialog;
use adw::prelude::*;
use gtk::{Button, GestureClick, GestureZoom, Overlay, ProgressBar};
use gtk::EventSequenceState::Claimed;
use gtk::Orientation::Vertical;
use gtk::PropagationPhase::Capture;
use id3::{Tag, TagLike};
use id3::ErrorKind::NoTag;
use id3::v1v2::write_to_path;
use id3::Version::Id3v24;
use log::error;
use crate::body::collection::model::Collection;
use crate::common::state::State;
use crate::common::StyledWidget;
use crate::common::util::Plural;
use crate::song::{Song, WithPath};

pub(in crate::body) struct MergeState {
    entity: &'static str,
    state: Rc<State>,
    title: Arc<String>,
    subtitle: Rc<String>,
    merging: Cell<bool>,
    selected_for_merge: RefCell<HashSet<gtk::Box>>,
    cancel_button: Button,
    merge_button: Button,
    pub merge_menu_button: Button,
}

trait MergeButton {
    fn disable(&self);
}

impl MergeButton for Button {
    fn disable(&self) {
        self.set_sensitive(false);
        self.set_tooltip_text(Some("Select 2 or more"));
    }
}

#[derive(Copy, Clone)]
pub(in crate::body) enum EntityType {
    Artist,
    Album,
}

pub(in crate::body) struct Entity {
    pub string: &'static str,
    pub entity_type: EntityType,
}

pub(in crate::body) const KEY: &'static str = "key";

impl MergeState {
    pub(in crate::body) fn new<G: Fn(Vec<Option<String>>, bool) -> Vec<(Song, Collection)> + Send + Clone + 'static,
        M: Fn(Song, Arc<String>) + Send + Clone + 'static>(merge_entity: Entity, state: Rc<State>, title: Arc<String>,
        subtitle: Rc<String>, get_songs: G, merge: M) -> Rc<Self> {
        let cancel_button = Button::builder().label("Cancel").build();
        let merge_button = Button::builder().label("Merge").build().suggested_action();
        merge_button.disable();
        let Entity { string, entity_type } = merge_entity;
        let merge_menu_button = Button::builder().label(format!("Merge {}s", string)).build();
        let heading = format!("Choose the correct {} name", string);
        let this = Rc::new(MergeState {
            entity: string,
            state: state.clone(),
            title,
            subtitle,
            merging: Cell::new(false),
            selected_for_merge: RefCell::new(HashSet::new()),
            cancel_button: cancel_button.clone(),
            merge_button: merge_button.clone(),
            merge_menu_button: merge_menu_button.clone(),
        });
        cancel_button.connect_clicked({
            let this = this.clone();
            move |_| {
                for row in this.selected_for_merge.borrow().iter() {
                    row.unset_background_accent();
                }
                this.end_merge();
            }
        });
        merge_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let gtk_box = gtk::Box::builder().orientation(Vertical).build();
                let overlay = Overlay::builder().child(&gtk_box).build();
                let dialog = MessageDialog::builder().heading(&heading).title(&heading).modal(true)
                    .extra_child(&overlay).transient_for(&state.window).build();
                let entities = this.selected_for_merge.borrow().iter().map(|entity| {
                    unsafe { entity.data::<Arc<String>>(KEY).map(|it| { it.as_ref().clone() }) }
                }).collect::<Vec<_>>();
                let has_none = entities.contains(&None);
                let entities = entities.into_iter().filter_map(|it| { it }).collect::<Vec<_>>();
                for entity in entities.clone() {
                    let button = Button::builder().label(entity.deref()).build().flat();
                    gtk_box.append(&button);
                    button.connect_clicked({
                        let overlay = overlay.clone();
                        let get_songs = get_songs.clone();
                        let entities = entities.clone();
                        let merge = merge.clone();
                        let dialog = dialog.clone();
                        move |_| {
                            let progress_bar = ProgressBar::builder().hexpand(true).build();
                            overlay.add_overlay(&progress_bar);
                            let (sender, receiver) = channel::<f64>();
                            thread::spawn({
                                let get_songs = get_songs.clone();
                                let entities = entities.clone();
                                let entity = entity.clone();
                                let merge = merge.clone();
                                move || {
                                    let song_collections = get_songs(entities.iter().filter_map(|it| {
                                        (!Arc::ptr_eq(&it, &entity)).then_some(Some(it.to_string()))
                                    }).collect::<Vec<_>>(), has_none);
                                    let total = song_collections.len();
                                    for (i, (song, collection)) in song_collections.into_iter().enumerate() {
                                        let current_path = (&song, &collection).path();
                                        let tag = match Tag::read_from_path(&current_path) {
                                            Ok(tag) => { Some(tag) }
                                            Err(error) => {
                                                if let NoTag = error.kind {
                                                    Some(Tag::new())
                                                } else {
                                                    error!("error reading tags on file [{:?}] while trying to set [{}] \
                                                    [{}] [{}]", current_path, string, entity.deref(), error);
                                                    None
                                                }
                                            }
                                        };
                                        if let Some(mut tag) = tag {
                                            match entity_type {
                                                EntityType::Artist => { tag.set_artist(entity.deref()); }
                                                EntityType::Album => { tag.set_album(entity.deref()); }
                                            }
                                            write_to_path(current_path, &tag, Id3v24).unwrap();
                                            merge(song, entity.clone());
                                        }
                                        sender.send(i as f64 / total as f64).unwrap();
                                    }
                                }
                            });
                            timeout_add_local(Duration::from_millis(500), {
                                let dialog = dialog.clone();
                                move || {
                                    let mut merge_progress: Option<f64> = None;
                                    loop {
                                        match receiver.try_recv() {
                                            Err(Empty) => { break; }
                                            Err(Disconnected) => {
                                                dialog.close();
                                                return Break;
                                            }
                                            Ok(fraction) => { merge_progress = Some(fraction); }
                                        }
                                    }
                                    if let Some(fraction) = merge_progress { progress_bar.set_fraction(fraction); }
                                    Continue
                                }
                            });
                        }
                    });
                }
                dialog.add_response("cancel", "_Cancel");
                dialog.present();
                this.end_merge();
            }
        });
        merge_menu_button.connect_clicked({
            let this = this.clone();
            move |_| {
                this.start_merge();
                this.state.menu_button.popdown();
            }
        });
        this
    }
    fn start_merge(&self) {
        if !self.merging.get() {
            self.merging.set(true);
            self.state.header_bar.remove(&self.state.back_button);
            self.state.header_bar.pack_start(&self.cancel_button);
            self.state.header_bar.remove(&self.state.menu_button);
            self.state.header_bar.pack_end(&self.merge_button);
            self.state.window_title.set_title(&format!("Merging {}s", self.entity));
        }
    }
    fn end_merge(&self) {
        self.state.header_bar.remove(&self.cancel_button);
        self.state.header_bar.pack_start(&self.state.back_button);
        self.state.header_bar.remove(&self.merge_button);
        self.state.header_bar.pack_end(&self.state.menu_button);
        self.state.window_title.set_title(&self.title);
        self.state.window_title.set_subtitle(&self.subtitle);
        self.selected_for_merge.borrow_mut().clear();
        self.merging.set(false);
    }
    fn update_selected_count(&self) {
        let count = self.selected_for_merge.borrow().len();
        self.state.window_title.set_subtitle(&format!("{} selected", count.number_plural(&self.entity)));
        if count > 1 {
            self.merge_button.set_sensitive(true);
            self.merge_button.set_tooltip_text(None);
        } else {
            self.merge_button.disable();
        }
    }
    fn select_row_for_merge(&self, row: &gtk::Box) {
        self.selected_for_merge.borrow_mut().insert(row.clone());
        row.set_background_accent();
        self.update_selected_count();
    }
    pub(in crate::body) fn handle_click<F: Fn() + 'static>(self: Rc<Self>, row: &gtk::Box, on_click: F) {
        let gesture_click = GestureClick::new();
        gesture_click.connect_pressed({
            let this = self.clone();
            let row = row.clone();
            move |_, _, _, _| { if !this.merging.get() { row.set_background_accent(); } }
        });
        gesture_click.connect_stopped({
            let this = self.clone();
            let row = row.clone();
            move |_| { if !this.merging.get() { row.unset_background_accent(); } }
        });
        gesture_click.connect_released({
            let row = row.clone();
            move |_, _, x, y| {
                if self.merging.get() {
                    if self.selected_for_merge.borrow().contains(&row) {
                        self.selected_for_merge.borrow_mut().remove(&row);
                        self.update_selected_count();
                        row.unset_background_accent();
                    } else {
                        self.select_row_for_merge(&row);
                    }
                } else {
                    if row.contains(x, y) { on_click(); }
                    row.unset_background_accent();
                }
            }
        });
        row.add_controller(gesture_click);
    }
    pub(in crate::body) fn handle_pinch(self: Rc<Self>, entities_box: gtk::Box) -> gtk::Box {
        let gesture_zoom = GestureZoom::builder().propagation_phase(Capture).build();
        gesture_zoom.connect_scale_changed({
            let entities_box = entities_box.clone();
            let this = self.clone();
            move |gesture, scale| {
                if scale < 1.0 {
                    gesture.set_state(Claimed);
                    let bounding_box = gesture.bounding_box().unwrap();
                    let mut top_found = false;
                    if let Some(mut child) = entities_box.first_child() {
                        loop {
                            if let Ok(row) = child.clone().downcast::<gtk::Box>() {
                                if !top_found && row.allocation()
                                    .contains_point(bounding_box.x(), bounding_box.y()) {
                                    top_found = true;
                                    this.start_merge();
                                    this.select_row_for_merge(&row);
                                }
                                if top_found && row.allocation()
                                    .contains_point(bounding_box.x(), bounding_box.y() + bounding_box.height()) {
                                    this.select_row_for_merge(&row);
                                    break;
                                }
                            }
                            if let Some(next) = child.next_sibling() { child = next; } else { break; }
                        }
                    }
                }
            }
        });
        entities_box.add_controller(gesture_zoom);
        entities_box
    }
}
