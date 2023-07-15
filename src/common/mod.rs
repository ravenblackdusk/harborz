use adw::gdk::pango::{AttrInt, AttrList, EllipsizeMode, Weight};
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
