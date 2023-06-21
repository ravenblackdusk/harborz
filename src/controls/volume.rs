use std::rc::Rc;
use diesel::{ExpressionMethods, RunQueryDsl, update};
use gtk::*;
use gtk::prelude::{BoxExt, ButtonExt, RangeExt};
use Orientation::Vertical;
use crate::common::gtk_box;
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::config::dsl::config;
use crate::schema::config::volume;

const VOLUME_STEP: f32 = 20.0;

pub(in crate::controls) fn volume_button() -> Rc<MenuButton> {
    let gtk_box = gtk_box(Vertical);
    let button = Rc::new(MenuButton::builder().popover(&Popover::builder().child(&gtk_box).build()).build());
    let scale = Rc::new(Scale::builder().orientation(Vertical).inverted(true).height_request(100).build());
    let increase_volume = Button::builder().icon_name("list-add").build();
    let decrease_volume = Button::builder().icon_name("list-remove").build();
    gtk_box.append(&increase_volume);
    gtk_box.append(&*scale);
    gtk_box.append(&decrease_volume);
    scale.set_range(0.0, 100.0);
    update_scale_and_button(&scale, &button, get_volume().expect("should be able to get volume"));
    scale.connect_change_value({
        let button = button.clone();
        move |scale, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                update_volume(scale, &button, |_| { value as f32 });
            }
            Inhibit(true)
        }
    });
    increase_volume.connect_clicked({
        let scale = scale.clone();
        let button = button.clone();
        move |_| {
            update_volume(&scale, &button, |current_volume| { 100.0_f32.min(current_volume + VOLUME_STEP) });
        }
    });
    decrease_volume.connect_clicked({
        let button = button.clone();
        move |_| {
            update_volume(&scale, &button, |current_volume| { 0.0_f32.max(current_volume - VOLUME_STEP) });
        }
    });
    button
}

fn get_volume() -> anyhow::Result<f32> {
    Ok(config.get_result::<Config>(&mut get_connection())?.volume)
}

fn update_scale_and_button(scale: &Scale, button: &MenuButton, value: f32) {
    scale.set_value(value as f64);
    button.set_icon_name(match value {
        i if i <= 0.0 => "audio-volume-muted",
        i if i <= 50.0 => "audio-volume-low",
        i if i < 100.0 => "audio-volume-medium",
        _ => "audio-volume-high",
    });
}

fn update_volume_internal<F>(scale: &Scale, button: &MenuButton, update_volume_value: F) -> anyhow::Result<()>
    where F: Fn(f32) -> f32 {
    let updated_volume = update_volume_value(get_volume()?);
    update(config).set(volume.eq(updated_volume)).execute(&mut get_connection())?;
    Ok(update_scale_and_button(scale, button, updated_volume))
}

fn update_volume<F>(scale: &Scale, button: &MenuButton, update_volume_value: F) where F: Fn(f32) -> f32 {
    update_volume_internal(scale, button, update_volume_value).expect("should be able to update volume");
}
