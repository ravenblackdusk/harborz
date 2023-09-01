use std::cell::Cell;
use std::path::PathBuf;
use std::time::Duration;
use adw::gdk::pango::{AttrInt, AttrList, EllipsizeMode, FontScale::Subscript, Weight, Weight::Bold};
use adw::glib::{timeout_add_local_once, Value};
use adw::prelude::*;
use gtk::{Box, Image, Orientation, ScrolledWindow, Widget};
use gtk::builders::{BoxBuilder, LabelBuilder};
use crate::common::constant::ACCENT_BG;

pub mod util;
pub mod constant;
pub mod state;
pub mod action;

pub fn box_builder() -> BoxBuilder {
    Box::builder().spacing(4).margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
}

pub fn gtk_box(orientation: Orientation) -> Box {
    box_builder().orientation(orientation).build()
}

pub trait StyledLabelBuilder {
    fn ellipsized(self) -> Self;
    fn margin_ellipsized(self, i: i32) -> Self;
    fn weight(self, weight: Weight) -> Self;
    fn bold(self) -> Self;
    fn subscript(self) -> Self;
    fn bold_subscript(self) -> Self;
}

impl StyledLabelBuilder for LabelBuilder {
    fn ellipsized(self) -> Self {
        self.hexpand(true).xalign(0.0).max_width_chars(1).ellipsize(EllipsizeMode::End)
    }
    fn margin_ellipsized(self, margin: i32) -> Self {
        self.ellipsized().margin_start(margin).margin_end(margin)
    }
    fn weight(self, weight: Weight) -> Self {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_weight(weight));
        self.attributes(&attr_list)
    }
    fn bold(self) -> Self {
        self.weight(Bold)
    }
    fn subscript(self) -> Self {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_font_scale(Subscript));
        self.attributes(&attr_list)
    }
    fn bold_subscript(self) -> Self {
        let attr_list = AttrList::new();
        attr_list.insert(AttrInt::new_font_scale(Subscript));
        attr_list.insert(AttrInt::new_weight(Bold));
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

pub struct IconType {
    icon_name: &'static str,
}

pub const SONG_ICON: IconType = IconType { icon_name: "audio-x-generic" };
pub const ALBUM_ICON: IconType = IconType { icon_name: "folder-music" };

pub trait ImagePathBuf {
    fn set_cover(&self, cover: &PathBuf, icon_type: IconType) -> &Self;
}

impl ImagePathBuf for Image {
    fn set_cover(&self, cover: &PathBuf, icon_type: IconType) -> &Self {
        if cover.exists() { self.set_from_file(Some(&cover)); } else { self.set_icon_name(Some(icon_type.icon_name)); }
        self
    }
}

pub trait StyledWidget {
    fn set_name(&self, name: impl Into<Value>);
    fn set_background_accent(&self);
    fn unset_background_accent(&self);
    fn with_css_class(self, css_class: &str) -> Self;
    fn numeric(self) -> Self;
    fn flat(self) -> Self;
    fn suggested_action(self) -> Self;
    fn destructive_action(self) -> Self;
    fn osd(self) -> Self;
}

impl<W: IsA<Widget>> StyledWidget for W {
    fn set_name(&self, name: impl Into<Value>) {
        self.set_property("name", name);
    }
    fn set_background_accent(&self) {
        self.set_name(ACCENT_BG);
    }
    fn unset_background_accent(&self) {
        self.set_name(None::<String>);
    }
    fn with_css_class(self, css_class: &str) -> Self {
        self.add_css_class(css_class);
        self
    }
    fn numeric(self) -> Self {
        self.with_css_class("numeric")
    }
    fn flat(self) -> Self {
        self.with_css_class("flat")
    }
    fn suggested_action(self) -> Self {
        self.with_css_class("suggested-action")
    }
    fn destructive_action(self) -> Self {
        self.with_css_class("destructive-action")
    }
    fn osd(self) -> Self {
        self.with_css_class("osd")
    }
}
