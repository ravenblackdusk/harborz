use std::cell::RefCell;
use adw::gio::{SimpleAction, SimpleActionGroup};
use adw::glib::Variant;
use adw::prelude::*;
use adw::Window;
use gtk::{Button, CheckButton, Label, Overlay, ScrolledWindow, Separator, Widget};
use gtk::Align::Center;
use gtk::Orientation::Vertical;
use crate::common::StyledWidget;

const DIALOG: &'static str = "dialog";
const CHOOSE_ENTITY: &'static str = "choose-correct-entity";

fn check_button<T: ToVariant>(entity: &(impl IsA<Widget>, Option<impl AsRef<T>>), gtk_box: &gtk::Box) -> CheckButton {
    let (widget, target) = entity;
    let check_button_builder = CheckButton::builder().child(widget).action_name(format!("{DIALOG}.{CHOOSE_ENTITY}"));
    let check_button = if let Some(target) = target {
        check_button_builder.action_target(&target.as_ref().to_variant())
    } else {
        check_button_builder
    }.build();
    gtk_box.append(&check_button);
    gtk_box.append(&Separator::builder().hexpand(true).build());
    check_button
}

pub fn check_button_dialog<T: ToVariant + StaticVariantType, F: FnMut(&Overlay, Variant, &Window) + 'static>(
    heading: &str, description: Option<&str>, entities: &Vec<(impl IsA<Widget>, Option<impl AsRef<T>>)>,
    choose_button_label: &str, default: T, css_class: &'static str, on_click: RefCell<F>) -> Vec<CheckButton> {
    let main_box = gtk::Box::builder().orientation(Vertical).spacing(4)
        .margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();
    let scrolled_window = ScrolledWindow::builder().child(&main_box)
        .propagate_natural_width(true).propagate_natural_height(true).build();
    let overlay = Overlay::builder().child(&scrolled_window).build();
    let dialog = Window::builder().title(heading).modal(true).content(&overlay).build();
    main_box.append(&Label::new(Some(heading)).with_css_class("heading"));
    if let Some(description) = description {
        main_box.append(&Label::new(Some(description)));
    }
    let check_button_box = gtk::Box::builder().orientation(Vertical).margin_top(16).margin_bottom(16).build();
    main_box.append(&check_button_box);
    check_button_box.append(&Separator::builder().hexpand(true).build());
    let first_check_button = check_button(&entities[0], &check_button_box);
    let mut check_buttons = entities[1..].iter().map(|entity| {
        let check_button = check_button(entity, &check_button_box);
        check_button.set_group(Some(&first_check_button));
        check_button
    }).collect::<Vec<_>>();
    let button_box = gtk::Box::builder().spacing(16).halign(Center).build();
    main_box.append(&button_box);
    let cancel_button = Button::builder().label("Cancel").build();
    button_box.append(&cancel_button);
    cancel_button.connect_clicked({
        let dialog = dialog.clone();
        move |_| { dialog.close(); }
    });
    let choose_button = Button::builder().label(choose_button_label).sensitive(false).build();
    button_box.append(&choose_button);
    let action_group = SimpleActionGroup::new();
    dialog.insert_action_group(DIALOG, Some(&action_group));
    let action = SimpleAction::new_stateful(CHOOSE_ENTITY, Some(&T::static_variant_type()), &default.to_variant());
    action_group.add_action(&action);
    action.connect_activate({
        let choose_button = choose_button.clone();
        move |action, state| {
            action.set_state(state.unwrap());
            choose_button.clone().with_css_class(css_class).set_sensitive(true);
        }
    });
    choose_button.connect_clicked({
        let dialog = dialog.clone();
        move |choose_button| {
            choose_button.set_sensitive(false);
            cancel_button.set_sensitive(false);
            on_click.borrow_mut()(&overlay, action.state().unwrap(), &dialog);
        }
    });
    dialog.present();
    check_buttons.insert(0, first_check_button);
    check_buttons
}
