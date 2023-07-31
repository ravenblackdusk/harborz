use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use adw::glib::signal::Inhibit;
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_into, QueryDsl, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Button, MenuButton, Popover, ScrolledWindow};
use gtk::Align::Fill;
use gtk::glib::ExitCode;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use NavigationType::History;
use crate::body::{Body, BodyType, BodyTable, NavigationType};
use crate::body::NavigationType::SongSelected;
use crate::common::{AdjustableScrolledWindow, gtk_box};
use crate::common::constant::APP_ID;
use crate::config::Config;
use crate::controls::media_controls;
use crate::controls::playbin::{PLAYBIN, Playbin};
use crate::db::get_connection;
use crate::schema::config::{current_song_position, maximized, window_height, window_width};
use crate::schema::config::dsl::config as config_table;
use crate::schema::bodies::{body_type, navigation_type, query1, query2, scroll_adjustment};
use crate::schema::bodies::dsl::bodies;

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
        let history_bodies = bodies.filter(navigation_type.eq(History)).get_results::<BodyTable>(&mut get_connection())
            .unwrap();
        let empty_history = history_bodies.is_empty();
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let window = ApplicationWindow::builder().application(application).content(&main_box)
            .default_width(config.window_width).default_height(config.window_height).maximized(config.maximized == 1)
            .build();
        let window_title = WindowTitle::builder().title("Harborz").subtitle("Artists").build();
        let history: Rc<RefCell<Vec<(Rc<Body>, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let song_selected_body: Rc<RefCell<Option<Rc<Body>>>> = Rc::new(RefCell::new(None));
        let bar = HeaderBar::builder().title_widget(&window_title).build();
        main_box.append(&bar);
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        main_box.append(&scrolled_window);
        let back_button = Button::builder().icon_name("go-previous-symbolic").tooltip_text("Home")
            .visible(history_bodies.len() > 1).build();
        bar.pack_start(&back_button);
        let media_controls = media_controls(song_selected_body.clone(), &window_title, &scrolled_window,
            history.clone(), &Some(back_button.clone()));
        main_box.append(&media_controls);
        *song_selected_body.borrow_mut() = bodies.filter(navigation_type.eq(SongSelected)).limit(1)
            .get_result::<BodyTable>(&mut get_connection()).ok().map(|body_table| {
            let body = Body::from_body_table(&body_table, &window_title, &scrolled_window, history.clone(),
                &media_controls, &back_button, &window);
            body.scroll_adjustment.set(body_table.scroll_adjustment);
            Rc::new(body)
        });
        back_button.connect_clicked({
            let history = history.clone();
            let window_title = window_title.clone();
            let scrolled_window = scrolled_window.clone();
            move |back_button| {
                let mut history = history.borrow_mut();
                history.pop();
                back_button.set_visible(history.len() > 1);
                if let Some((body, adjust_scroll)) = history.last() {
                    let Body { title, subtitle, widget, scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
                    window_title.set_title(title.as_str());
                    window_title.set_subtitle(subtitle.as_str());
                    scrolled_window.set_child(Some((**widget).as_ref()));
                    if *adjust_scroll { scrolled_window.adjust(&body_scroll_adjustment); }
                }
            }
        });
        let menu = gtk_box(Vertical);
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text("Menu")
            .popover(&Popover::builder().child(&menu).build()).build();
        bar.pack_end(&menu_button);
        let collection_button = Button::builder().label("Collection").build();
        menu.append(&collection_button);
        collection_button.connect_clicked({
            let history = history.clone();
            let window = window.clone();
            let window_title = window_title.clone();
            let scrolled_window = scrolled_window.clone();
            let menu_button = menu_button.clone();
            let back_button = back_button.clone();
            move |_| {
                if history.borrow().last().unwrap().0.body_type != BodyType::Collections {
                    Rc::new(Body::collections(&window))
                        .set(&window_title, &scrolled_window, history.clone(), &Some(back_button.clone()));
                }
                menu_button.popdown();
            }
        });
        for body_table in history_bodies {
            Body::from_body_table(&body_table, &window_title, &scrolled_window, history.clone(), &media_controls,
                &back_button, &window,
            ).put_to_history(body_table.scroll_adjustment, history.clone());
        }
        if empty_history {
            Rc::new(Body::artists(&window_title, &scrolled_window, history.clone(), &media_controls,
                &Some(back_button.clone()))
            ).set(&window_title, &scrolled_window, history.clone(), &None);
        } else if let Some((body, _)) = history.borrow().last() {
            let Body { title, subtitle, widget, scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
            window_title.set_title(title.as_str());
            window_title.set_subtitle(subtitle.as_str());
            scrolled_window.set_child(Some((**widget).as_ref()));
            scrolled_window.adjust(&body_scroll_adjustment);
        }
        window.connect_close_request({
            let history = history.clone();
            let scrolled_window = scrolled_window.clone();
            move |window| {
                let (width, height) = window.default_size();
                update(config_table).set((window_width.eq(width), window_height.eq(height),
                    maximized.eq(if window.is_maximized() { 1 } else { 0 }),
                    current_song_position.eq(PLAYBIN.get_position().unwrap_or(0) as i64)
                )).execute(&mut get_connection()).unwrap();
                delete(bodies).execute(&mut get_connection()).unwrap();
                let history = history.borrow();
                if let Some((body, _)) = history.last() {
                    body.scroll_adjustment.set(scrolled_window.get_adjustment());
                }
                insert_into(bodies).values(
                    history.iter().map(|(body, _)| { (body, History) })
                        .chain(song_selected_body.borrow().iter().map(|body| { (body, SongSelected) }))
                        .map(|(body, body_navigation_type)| {
                            let Body {
                                query1: body_query1, query2: body_query2, body_type: body_body_type,
                                scroll_adjustment: body_scroll_adjustment, ..
                            } = body.deref();
                            (query1.eq(body_query1.clone().map(|it| { String::from(it.as_str()) })),
                                query2.eq(body_query2.clone().map(|it| { String::from(it.as_str()) })),
                                body_type.eq(body_body_type), scroll_adjustment.eq(body_scroll_adjustment.get()),
                                navigation_type.eq(body_navigation_type)
                            )
                        }).collect::<Vec<_>>()
                ).execute(&mut get_connection()).unwrap();
                Inhibit(false)
            }
        });
        window.present();
    });
    Ok(application.run())
}
