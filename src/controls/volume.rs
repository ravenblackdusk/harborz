use diesel::{ExpressionMethods, RunQueryDsl, update};
use gtk::*;
use gtk::prelude::{BoxExt, ButtonExt, RangeExt, WidgetExt};
use Orientation::Vertical;
use crate::common::gtk_box;
use crate::config::Config;
use crate::db::get_connection;
use crate::schema::config::dsl::config;
use crate::schema::config::volume;

const VOLUME_STEP: f64 = 0.2;

pub(in crate::controls) fn volume_button<F: Fn(f64) + Clone + 'static>(on_volume_change: F) -> MenuButton {
    let gtk_box = gtk_box(Vertical);
    let button = MenuButton::builder().popover(&Popover::builder().child(&gtk_box).build()).build();
    let scale = Scale::builder().orientation(Vertical).inverted(true).height_request(100).build();
    let increase_volume = Button::builder().icon_name("list-add").build();
    let decrease_volume = Button::builder().icon_name("list-remove").build();
    gtk_box.append(&increase_volume);
    gtk_box.append(&scale);
    gtk_box.append(&decrease_volume);
    scale.set_range(0.0, 1.0);
    let cloned_scale = scale.clone();
    let cloned_button = button.clone();
    let update_ui = move |value| {
        on_volume_change(value);
        scale.set_value(value);
        cloned_button.set_icon_name(match value {
            i if i <= 0.0 => "audio-volume-muted",
            i if i <= 0.5 => "audio-volume-low",
            i if i < 1.0 => "audio-volume-medium",
            _ => "audio-volume-high",
        });
        let percentage: String;
        cloned_button.set_tooltip_text(Some(if value <= 0.0 {
            "Muted"
        } else {
            percentage = format!("{}%", (value * 100.0) as u8);
            &percentage
        }));
    };
    update_ui(get_volume());
    let update_volume = move |value| {
        update(config).set(volume.eq(value as f32)).execute(&mut get_connection()).unwrap();
        update_ui(value)
    };
    cloned_scale.connect_change_value({
        let update_volume = update_volume.clone();
        move |_, scroll_type, value| {
            if scroll_type == ScrollType::Jump {
                update_volume(value);
            }
            Inhibit(true)
        }
    });
    increase_volume.connect_clicked({
        let update_volume = update_volume.clone();
        move |_| { update_volume((get_volume() + VOLUME_STEP).min(1.0)); }
    });
    decrease_volume.connect_clicked(move |_| { update_volume((get_volume() - VOLUME_STEP).max(0.0)); });
    button
}

fn get_volume() -> f64 {
    config.get_result::<Config>(&mut get_connection()).unwrap().volume as f64
}
