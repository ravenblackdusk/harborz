pub mod util;
pub mod wrapper;

use gtk::{Box, Orientation};
use gtk::builders::BoxBuilder;

pub fn box_builder() -> BoxBuilder {
    Box::builder().spacing(4).margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
}

pub fn gtk_box(orientation: Orientation) -> Box {
    box_builder().orientation(orientation).build()
}
