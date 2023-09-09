use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{channel, TryRecvError::*};
use std::thread;
use std::time::Duration;
use adw::glib::{ControlFlow::*, timeout_add_local, Variant};
use adw::prelude::*;
use adw::Window;
use diesel::{BoolExpressionMethods, QueryDsl, RunQueryDsl};
use gtk::{Button, CheckButton, GestureClick, GestureZoom, Label, Overlay, ProgressBar};
use gtk::EventSequenceState::Claimed;
use gtk::PropagationPhase::Capture;
use id3::ErrorKind::NoTag;
use id3::Tag;
use id3::v1v2::write_to_path;
use id3::Version::Id3v24;
use log::error;
use crate::body::collection::model::Collection;
use crate::body::merge::{KEY, MergeButton, MergeState, Query};
use crate::body::next_icon;
use crate::common::check_button_dialog::check_button_dialog;
use crate::common::constant::DESTRUCTIVE_ACTION;
use crate::common::state::State;
use crate::common::StyledWidget;
use crate::common::util::Plural;
use crate::db::get_connection;
use crate::schema::collections::dsl::collections;
use crate::schema::songs::dsl::songs;
#[allow(unused_imports)]
use crate::song::{Song, WithPath};

impl MergeState {
    pub(in crate::body) fn new<
        I: Fn(Vec<Option<String>>) -> Query + Send + Clone + 'static, N: Fn() -> Query + Send + Clone + 'static,
        T: Fn(&mut Tag, &str) + Send + Clone + 'static, M: Fn(Song, &str) + Send + Clone + 'static
    >(string: &'static str, state: Rc<State>, title: Arc<String>, subtitle: Rc<String>, entities_box: gtk::Box,
        get_in_filter: I, is_null: N, set_tag: T, merge: M) -> Rc<Self> {
        let cancel_button = Button::builder().label("Cancel").build();
        let merge_button = Button::builder().label("Merge").build().suggested_action();
        merge_button.disable();
        let heading = format!("Merge {string}s");
        let description = format!("Choose the correct {string} name");
        let merge_menu_button = Button::builder().label(&heading).build();
        let this = Rc::new(MergeState {
            entity: string,
            state: state.clone(),
            title,
            subtitle,
            entities_box,
            merging: Cell::new(false),
            selected_for_merge: RefCell::new(HashSet::new()),
            cancel_button: cancel_button.clone(),
            merge_button: merge_button.clone(),
            merge_menu_button: merge_menu_button.clone(),
        });
        cancel_button.connect_clicked({
            let this = this.clone();
            move |_| { this.end_merge(); }
        });
        merge_button.connect_clicked({
            let this = this.clone();
            move |_| {
                let entities = this.selected_for_merge.borrow().iter().map(|entity| {
                    unsafe { entity.data::<Arc<String>>(KEY).map(|it| { it.as_ref().clone() }) }
                }).collect::<Vec<_>>();
                let has_none = entities.contains(&None);
                let entities = entities.into_iter().filter_map(|it| { it }).collect::<Vec<_>>();
                check_button_dialog(&heading, Some(&description),
                    &entities.clone().into_iter().map(|it| { (Label::new(Some(&it)), Some(it)) }).collect::<Vec<_>>(),
                    "Merge", String::from(""), DESTRUCTIVE_ACTION, RefCell::new({
                        let get_in_filter = get_in_filter.clone();
                        let entities = entities.clone();
                        let is_null = is_null.clone();
                        let set_tag = set_tag.clone();
                        let merge = merge.clone();
                        let this = this.clone();
                        move |overlay: &Overlay, variant: Variant, dialog: &Window| {
                            let progress_bar = ProgressBar::builder().hexpand(true).build().osd();
                            overlay.add_overlay(&progress_bar);
                            let (sender, receiver) = channel::<f64>();
                            thread::spawn({
                                let get_in_filter = get_in_filter.clone();
                                let entities = entities.clone();
                                let is_null = is_null.clone();
                                let set_tag = set_tag.clone();
                                let merge = merge.clone();
                                move || {
                                    let entity = variant.str().unwrap();
                                    let in_filter = get_in_filter(entities.iter().filter_map(|it| {
                                        (**it != entity).then_some(Some((**it).to_owned()))
                                    }).collect::<Vec<_>>());
                                    let statement = songs.inner_join(collections).into_boxed();
                                    let song_collections = if has_none {
                                        statement.filter(in_filter.or(is_null()))
                                    } else {
                                        statement.filter(in_filter)
                                    }.get_results::<(Song, Collection)>(&mut get_connection()).unwrap();
                                    let total = song_collections.len();
                                    for (i, (song, collection)) in song_collections.into_iter().enumerate() {
                                        let current_path = (&song, &collection).path();
                                        let tag = match Tag::read_from_path(&current_path) {
                                            Ok(tag) => { Some(tag) }
                                            Err(error) => {
                                                if let NoTag = error.kind {
                                                    Some(Tag::new())
                                                } else {
                                                    error!("error reading tags on file [{current_path:?}] while trying \
                                                    to set [{string}] [{entity}] [{error}]");
                                                    None
                                                }
                                            }
                                        };
                                        if let Some(mut tag) = tag {
                                            set_tag(&mut tag, &entity);
                                            write_to_path(current_path, &tag, Id3v24).unwrap();
                                            merge(song, &entity);
                                        }
                                        sender.send(i as f64 / total as f64).unwrap();
                                    }
                                }
                            });
                            timeout_add_local(Duration::from_millis(500), {
                                let this = this.clone();
                                let dialog = dialog.clone();
                                move || {
                                    let mut merge_progress: Option<f64> = None;
                                    loop {
                                        match receiver.try_recv() {
                                            Err(Empty) => { break; }
                                            Err(Disconnected) => {
                                                this.end_merge();
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
                    }),
                );
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
    fn iterate_rows(&self, mut do_with_row: impl FnMut(gtk::Box) -> bool) {
        if let Some(mut child) = self.entities_box.first_child() {
            loop {
                if let Ok(row) = child.clone().downcast::<gtk::Box>() { if do_with_row(row) { break; } }
                if let Some(next) = child.next_sibling() { child = next; } else { break; }
            }
        }
    }
    fn start_merge(&self) {
        if !self.merging.get() {
            self.merging.set(true);
            self.state.header_bar.remove(&self.state.back_button);
            self.state.header_bar.pack_start(&self.cancel_button);
            self.state.header_bar.remove(&self.state.menu_button);
            self.state.header_bar.pack_end(&self.merge_button);
            self.state.window_actions.change_window_title.activate(format!("Merging {}s", self.entity));
            self.update_selected_count();
            self.iterate_rows(|row| {
                row.remove(&row.last_child().unwrap());
                row.append(&CheckButton::builder().margin_end(8).build());
                false
            });
        }
    }
    fn end_merge(&self) {
        self.state.header_bar.remove(&self.cancel_button);
        self.state.header_bar.pack_start(&self.state.back_button);
        self.state.header_bar.remove(&self.merge_button);
        self.state.header_bar.pack_end(&self.state.menu_button);
        self.state.window_actions.change_window_title.activate(&*self.title);
        self.state.window_actions.change_window_subtitle.activate(&*self.subtitle);
        self.selected_for_merge.borrow_mut().clear();
        self.merging.set(false);
        self.iterate_rows(|row| {
            row.remove(&row.last_child().unwrap());
            row.append(&next_icon());
            false
        });
    }
    fn update_selected_count(&self) {
        let count = self.selected_for_merge.borrow().len();
        self.state.window_actions.change_window_subtitle
            .activate(format!("{} selected", count.number_plural(&self.entity)));
        if count > 1 {
            self.merge_button.set_sensitive(true);
            self.merge_button.set_tooltip_text(None);
        } else {
            self.merge_button.disable();
        }
    }
    fn select_row_for_merge(&self, row: &gtk::Box) {
        self.selected_for_merge.borrow_mut().insert(row.clone());
        row.last_child().and_downcast::<CheckButton>().unwrap().set_active(true);
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
                        row.last_child().and_downcast::<CheckButton>().unwrap().set_active(false);
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
    pub(in crate::body) fn handle_pinch(self: Rc<Self>) -> gtk::Box {
        let gesture_zoom = GestureZoom::builder().propagation_phase(Capture).build();
        gesture_zoom.connect_scale_changed({
            let this = self.clone();
            move |gesture, scale| {
                if scale < 1.0 {
                    gesture.set_state(Claimed);
                    let bounding_box = gesture.bounding_box().unwrap();
                    let mut top_found = false;
                    this.iterate_rows(|row| {
                        if !top_found && row.allocation().contains_point(bounding_box.x(), bounding_box.y()) {
                            top_found = true;
                            this.start_merge();
                            this.select_row_for_merge(&row);
                        }
                        if top_found && row.allocation()
                            .contains_point(bounding_box.x(), bounding_box.y() + bounding_box.height()) {
                            this.select_row_for_merge(&row);
                            return true;
                        }
                        false
                    });
                }
            }
        });
        self.entities_box.add_controller(gesture_zoom);
        self.entities_box.clone()
    }
}
