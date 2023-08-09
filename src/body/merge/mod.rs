use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;
use adw::MessageDialog;
use adw::prelude::*;
use gtk::{Button, GestureClick, GestureZoom};
use gtk::EventSequenceState::Claimed;
use gtk::Orientation::Vertical;
use gtk::PropagationPhase::Capture;
use crate::common::state::State;
use crate::common::StyledWidget;
use crate::common::util::Plural;

pub(in crate::body) struct MergeState {
    entity: &'static str,
    state: Rc<State>,
    title: Rc<String>,
    subtitle: Rc<String>,
    merging: Cell<bool>,
    selected_for_merge: RefCell<HashSet<gtk::Box>>,
    cancel_button: Button,
    merge_button: Button,
    pub merge_menu_button: Button,
}

impl MergeState {
    pub(in crate::body) fn new<F: Fn(Vec<Rc<String>>, Rc<String>, bool) + Clone + 'static>(entity: &'static str,
        state: Rc<State>, title: Rc<String>, subtitle: Rc<String>, on_merge: F) -> Rc<Self> {
        let cancel_button = Button::builder().label("Cancel").build();
        let merge_button = Button::builder().label("Merge").build().suggested_action();
        let merge_menu_button = Button::builder().label(format!("Merge {}s", entity)).build();
        let heading = format!("Choose the correct {} name", entity);
        let this = Rc::new(MergeState {
            entity,
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
                let dialog = MessageDialog::builder().heading(&heading).title(&heading).modal(true)
                    .extra_child(&gtk_box).transient_for(&state.window).build();
                let entities = this.selected_for_merge.borrow().iter().map(|entity| {
                    unsafe { entity.data::<Rc<String>>("key").map(|it| { it.as_ref().clone() }) }
                }).collect::<Vec<_>>();
                let has_none = entities.contains(&None);
                let entities = entities.into_iter().filter_map(|it| { it }).collect::<Vec<_>>();
                for entity in entities.clone() {
                    let button = Button::builder().label(entity.deref()).build().flat();
                    gtk_box.append(&button);
                    button.connect_clicked({
                        let on_merge = on_merge.clone();
                        let entities = entities.clone();
                        let dialog = dialog.clone();
                        move |_| {
                            on_merge(entities.clone(), entity.clone(), has_none);
                            dialog.close();
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
    fn update_merging_subtitle(&self) {
        self.state.window_title.set_subtitle(&format!("{} selected for merge",
            self.selected_for_merge.borrow().len().number_plural(&self.entity)));
    }
    fn select_row_for_merge(&self, row: &gtk::Box) {
        self.selected_for_merge.borrow_mut().insert(row.clone());
        row.set_background_accent();
        self.update_merging_subtitle();
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
                        self.update_merging_subtitle();
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
