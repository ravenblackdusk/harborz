use diesel::RunQueryDsl;
use gtk::*;
use gtk::prelude::RangeExt;
use Orientation::Vertical;
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::config::dsl::config;

pub(in crate::controls) fn volume_button() -> MenuButton {
    let scale = Scale::builder().orientation(Vertical).height_request(100).build();
    scale.set_range(0.0, 100.0);
    scale.set_value(config.get_result::<Config>(&mut get_connection()).expect("should be able to get config").volume as f64);
    let popover = Popover::builder().child(&scale).build();
    MenuButton::builder().popover(&popover).icon_name("audio-volume-medium").build()
}
