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
    let cloned_scale = scale.clone();
    let cloned_button = button.clone();
    let update_scale_and_button = move |value: f32| {
        scale.set_value(value as f64);
        cloned_button.set_icon_name(match value {
            i if i <= 0.0 => "audio-volume-muted",
            i if i <= 50.0 => "audio-volume-low",
            i if i < 100.0 => "audio-volume-medium",
            _ => "audio-volume-high",
        });
    };
    update_scale_and_button(get_volume());
    let update_volume = Rc::new(move |value: f32| {
        update(config).set(volume.eq(value)).execute(&mut get_connection()).unwrap();
        update_scale_and_button(value)
    });
    cloned_scale.connect_change_value({
        let update_volume = update_volume.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                update_volume(value as f32);
            }
            Inhibit(true)
        }
    });
    increase_volume.connect_clicked({
        let update_volume = update_volume.clone();
        move |_| { update_volume(100.0_f32.min(get_volume() + VOLUME_STEP)); }
    });
    decrease_volume.connect_clicked(move |_| { update_volume(0.0_f32.max(get_volume() - VOLUME_STEP)); });
    button
}

fn get_volume() -> f32 {
    config.get_result::<Config>(&mut get_connection()).unwrap().volume
}
