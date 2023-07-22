use std::cell::RefCell;
use std::rc::Rc;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use adw::glib::signal::Inhibit;
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_into, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Button, MenuButton, Popover, ScrolledWindow};
use gtk::Align::Fill;
use gtk::glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use crate::body::{Body, BodyType, HistoryBody};
use crate::common::{AdjustableScrolledWindow, gtk_box};
use crate::common::constant::APP_ID;
use crate::config::Config;
use crate::controls::media_controls;
use crate::controls::playbin::{PLAYBIN, Playbin};
use crate::db::get_connection;
use crate::schema::config::{current_song_position, maximized, window_height, window_width};
use crate::schema::config::dsl::config as config_table;
use crate::schema::history_bodies::{body_type, query, scroll_adjustment};
use crate::schema::history_bodies::dsl::history_bodies;

mod schema;
mod db;
mod controls;
mod common;
mod config;
mod body;
mod song;

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(|application| {
        let main_box = gtk::Box::builder().orientation(Vertical).valign(Fill).build();
        let bodies = history_bodies.get_results::<HistoryBody>(&mut get_connection()).unwrap();
        let empty_history = bodies.is_empty();
        let back_button = Button::builder().icon_name("go-previous-symbolic").tooltip_text("Home")
            .visible(bodies.len() > 1).build();
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let window = ApplicationWindow::builder().application(application).content(&main_box)
            .default_width(config.window_width).default_height(config.window_height).maximized(config.maximized == 1)
            .build();
        let media_controls = media_controls();
        let window_title = WindowTitle::builder().title("Harborz").subtitle("Artists").build();
        let bar = HeaderBar::builder().title_widget(&window_title).build();
        let collection_button = Button::builder().label("Collection").build();
        let menu = gtk_box(Vertical);
        menu.append(&collection_button);
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text("Menu")
            .popover(&Popover::builder().child(&menu).build()).build();
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        let history: Rc<RefCell<Vec<(Body, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        back_button.connect_clicked({
            let history = history.clone();
            let window_title = window_title.clone();
            let scrolled_window = scrolled_window.clone();
            move |back_button| {
                let mut history = history.borrow_mut();
                history.pop();
                back_button.set_visible(history.len() > 1);
                if let Some((Body { title, subtitle, widget, scroll_adjustment: body_scroll_adjustment, .. },
                    adjust_scroll)) = history.last() {
                    window_title.set_title(title.as_str());
                    window_title.set_subtitle(subtitle.as_str());
                    scrolled_window.set_child(Some((**widget).as_ref()));
                    if *adjust_scroll { scrolled_window.adjust(body_scroll_adjustment); }
                }
            }
        });
        collection_button.connect_clicked({
            let history = history.clone();
            let window = window.clone();
            let window_title = window_title.clone();
            let scrolled_window = scrolled_window.clone();
            let menu_button = menu_button.clone();
            let back_button = back_button.clone();
            move |_| {
                if history.borrow().last().unwrap().0.body_type != BodyType::Collections {
                    Body::collections(&window)
                        .set(&window_title, &scrolled_window, history.clone(), &Some(back_button.clone()));
                }
                menu_button.popdown();
            }
        });
        let mut artist: Option<String> = None;
        for HistoryBody { query: body_query, body_type: body_body_type, scroll_adjustment: body_scroll_adjustment, .. }
        in bodies {
            match body_body_type {
                BodyType::Artists => {
                    Body::artists(&window_title, &scrolled_window, history.clone(), &media_controls,
                        &Some(back_button.clone())
                    ).put_to_history(body_scroll_adjustment, history.clone());
                }
                BodyType::Albums => {
                    artist = body_query;
                    Body::albums(artist.clone(), &window_title, &scrolled_window, history.clone(), &media_controls)
                        .put_to_history(body_scroll_adjustment, history.clone());
                }
                BodyType::Songs => {
                    Body::songs(body_query, artist.clone().map(Rc::new), &media_controls)
                        .put_to_history(body_scroll_adjustment, history.clone());
                }
                BodyType::Collections => {
                    Body::collections(&window).put_to_history(body_scroll_adjustment, history.clone());
                }
            }
        }
        if empty_history {
            Body::artists(&window_title, &scrolled_window, history.clone(), &media_controls, &Some(back_button.clone()))
                .set(&window_title, &scrolled_window, history.clone(), &None);
        } else if let Some((Body { title, subtitle, widget, scroll_adjustment: body_scroll_adjustment, .. }, _))
            = history.borrow().last() {
            window_title.set_title(title.as_str());
            window_title.set_subtitle(subtitle.as_str());
            scrolled_window.set_child(Some((**widget).as_ref()));
            scrolled_window.adjust(body_scroll_adjustment);
        }
        main_box.append(&bar);
        main_box.append(&scrolled_window);
        main_box.append(&media_controls);
        bar.pack_start(&back_button);
        bar.pack_end(&menu_button);
        window.connect_close_request({
            let history = history.clone();
            let scrolled_window = scrolled_window.clone();
            move |window| {
                let (width, height) = window.default_size();
                update(config_table).set((window_width.eq(width), window_height.eq(height),
                    maximized.eq(if window.is_maximized() { 1 } else { 0 }),
                    current_song_position.eq(PLAYBIN.get_position().unwrap_or(0) as i64)
                )).execute(&mut get_connection()).unwrap();
                delete(history_bodies).execute(&mut get_connection()).unwrap();
                let history = history.borrow();
                if let Some((body, _)) = history.last() {
                    body.scroll_adjustment.set(scrolled_window.get_adjustment());
                }
                insert_into(history_bodies).values(history.iter().map(|(Body {
                    query: body_query, body_type: body_body_type, scroll_adjustment: body_scroll_adjustment, ..
                }, _)| {
                    (query.eq(body_query.clone().map(|it| { String::from(it.as_str()) })),
                        body_type.eq(body_body_type), scroll_adjustment.eq(body_scroll_adjustment.get()))
                }).collect::<Vec<_>>()).execute(&mut get_connection()).unwrap();
                Inhibit(false)
            }
        });
        window.present();
    });
    Ok(application.run())
}
