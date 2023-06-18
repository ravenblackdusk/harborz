use gtk::{Box, Orientation};

pub fn gtk_box(orientation: Orientation) -> Box {
    Box::builder().orientation(orientation).spacing(4)
        .margin_start(4).margin_end(4).margin_top(4).margin_bottom(4).build()
}
