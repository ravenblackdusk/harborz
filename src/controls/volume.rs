use std::any::Any;
use std::cmp::max;
use std::ops::Add;
use diesel::{AsChangeset, ExpressionMethods, RunQueryDsl, update};
use gtk::*;
use gtk::glib::clone;
use gtk::prelude::{BoxExt, ButtonExt, RangeExt};
use Orientation::Vertical;
use crate::common::gtk_box;
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::config::dsl::config;
use crate::schema::config::volume;

const VOLUME_STEP: f32 = 20.0;

pub(in crate::controls) fn volume_button() -> MenuButton {
    let scale = Scale::builder().orientation(Vertical).inverted(true).height_request(100).build();
    scale.set_range(0.0, 100.0);
    scale.set_value(get_volume().expect("should be able to get volume") as f64);
    scale.connect_change_value(|scale, scroll_type, value| {
        if scroll_type == ScrollType::Jump {
            update_volume(scale, |_| { value as f32 });
        }
        Inhibit(true)
    });
    let increase_volume = Button::builder().icon_name("list-add").build();
    increase_volume.connect_clicked(clone!(@weak scale => move |_| {
        update_volume(&scale, |current_volume| { 100.0_f32.min(current_volume + VOLUME_STEP) });
    }));
    let decrease_volume = Button::builder().icon_name("list-remove").build();
    decrease_volume.connect_clicked(clone!(@weak scale => move |_| {
        update_volume(&scale, |current_volume| { 0.0_f32.max(current_volume - VOLUME_STEP) });
    }));
    let gtk_box = gtk_box(Vertical);
    gtk_box.append(&increase_volume);
    gtk_box.append(&scale);
    gtk_box.append(&decrease_volume);
    MenuButton::builder().popover(&Popover::builder().child(&gtk_box).build())
        .icon_name("audio-volume-medium").build()
}

fn get_volume() -> anyhow::Result<f32> {
    Ok(config.get_result::<Config>(&mut get_connection())?.volume)
}

fn update_volume_internal<F>(scale: &Scale, update_volume_value: F) -> anyhow::Result<()> where F: Fn(f32) -> f32 {
    let updated_volume = update_volume_value(get_volume()?);
    update(config).set(volume.eq(updated_volume)).execute(&mut get_connection())?;
    Ok(scale.set_value(updated_volume as f64))
}

fn update_volume<F>(scale: &Scale, update_volume_value: F) where F: Fn(f32) -> f32 {
    update_volume_internal(scale, update_volume_value).expect("should be able to update volume");
}
