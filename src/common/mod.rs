use std::cell::Cell;
use std::time::Duration;
use adw::gdk::pango::{AttrInt, AttrList, EllipsizeMode, FontScale, Weight};
use adw::glib::timeout_add_local_once;
use gtk::{Box, Orientation, ScrolledWindow};
use gtk::builders::{BoxBuilder, LabelBuilder};
use gtk::prelude::AdjustmentExt;

pub mod util;
pub mod wrapper;
pub mod constant;

pub fn box_builder() -> BoxBuilder {
    Box::builder().spacing(4).margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
}

pub fn gtk_box(orientation: Orientation) -> Box {
    box_builder().orientation(orientation).build()
}

pub trait EllipsizedLabelBuilder {
    fn ellipsized(self) -> Self;
    fn margin_ellipsized(self, i: i32) -> Self;
}

impl EllipsizedLabelBuilder for LabelBuilder {
    fn ellipsized(self) -> Self {
        self.hexpand(true).xalign(0.0).max_width_chars(1).ellipsize(EllipsizeMode::End)
    }
    fn margin_ellipsized(self, margin: i32) -> Self {
        self.ellipsized().margin_start(margin).margin_end(margin)
    }
}

pub trait BoldLabelBuilder {
    fn bold(self) -> Self;
}

impl BoldLabelBuilder for LabelBuilder {
    fn bold(self) -> Self {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_weight(Weight::Bold));
        self.attributes(&attr_list)
    }
}

pub trait SubscriptLabelBuilder {
    fn subscript(self) -> LabelBuilder;
}

impl SubscriptLabelBuilder for LabelBuilder {
    fn subscript(self) -> LabelBuilder {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_font_scale(FontScale::Subscript));
        self.attributes(&attr_list)
    }
}

pub trait BoldSubscriptLabelBuilder {
    fn bold_subscript(self) -> LabelBuilder;
}

impl BoldSubscriptLabelBuilder for LabelBuilder {
    fn bold_subscript(self) -> LabelBuilder {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_font_scale(FontScale::Subscript));
        attr_list.insert(AttrInt::new_weight(Weight::Bold));
        self.attributes(&attr_list)
    }
}

pub trait AdjustableScrolledWindow {
    fn get_adjustment(&self) -> Option<f32>;
    fn adjust(&self, value: &Cell<Option<f32>>);
}

impl AdjustableScrolledWindow for ScrolledWindow {
    fn get_adjustment(&self) -> Option<f32> {
        Some(self.vadjustment().value() as f32)
    }
    fn adjust(&self, value: &Cell<Option<f32>>) {
        if let Some(value) = value.get() {
            timeout_add_local_once(Duration::from_millis(100), {
                let this = self.clone();
                move || { this.clone().vadjustment().set_value(value as f64); }
            });
        }
    }
}
