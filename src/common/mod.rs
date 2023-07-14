use adw::gdk::pango::EllipsizeMode;
use gtk::{Box, Orientation};
use gtk::builders::{BoxBuilder, LabelBuilder};

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
}

impl EllipsizedLabelBuilder for LabelBuilder {
    fn ellipsized(self) -> Self {
        self.margin_start(4).margin_end(4).hexpand(true).xalign(0.0).max_width_chars(1).ellipsize(EllipsizeMode::End)
    }
}
