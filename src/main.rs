use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use adw::glib::{ExitCode, Propagation};
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_into, QueryDsl, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Button, MenuButton, Popover, ScrolledWindow};
use gtk::Align::Fill;
use gtk::Orientation::Vertical;
use db::MIGRATIONS;
use NavigationType::History;
use crate::body::{Body, BodyTable, BodyType, NavigationType};
use crate::body::NavigationType::SongSelected;
use crate::common::{AdjustableScrolledWindow, gtk_box};
use crate::common::constant::{APP_ID, BACK_ICON};
use crate::config::Config;
use crate::db::get_connection;
use crate::now_playing::playbin::{PLAYBIN, Playbin};
use crate::schema::bodies::{body_type, navigation_type, query1, query2, scroll_adjustment};
use crate::schema::bodies::dsl::bodies;
use crate::schema::config::{current_song_position, maximized, window_height, window_width};
use crate::schema::config::dsl::config as config_table;

mod schema;
mod db;
mod now_playing;
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
        let header_body = gtk::Box::builder().orientation(Vertical).valign(Fill).build();
        let history_bodies = bodies.filter(navigation_type.eq(History)).get_results::<BodyTable>(&mut get_connection())
            .unwrap();
        let empty_history = history_bodies.is_empty();
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let window = ApplicationWindow::builder().application(application).content(&header_body)
            .default_width(config.window_width).default_height(config.window_height).maximized(config.maximized == 1)
            .build();
        let window_title = WindowTitle::builder().build();
        let history: Rc<RefCell<Vec<(Rc<Body>, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let song_selected_body: Rc<RefCell<Option<Rc<Body>>>> = Rc::new(RefCell::new(None));
        let header_bar = HeaderBar::builder().title_widget(&window_title).build();
        header_body.append(&header_bar);
        let body = gtk::Box::builder().orientation(Vertical).build();
        header_body.append(&body);
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        body.append(&scrolled_window);
        let back_button = Button::builder().icon_name(BACK_ICON).tooltip_text("Home").visible(history_bodies.len() > 1)
            .build();
        header_bar.pack_start(&back_button);
        let now_playing = now_playing::create(song_selected_body.clone(), &window_title, &scrolled_window,
            history.clone(), &back_button, &header_body, &body);
        body.append(&now_playing);
        *song_selected_body.borrow_mut() = bodies.filter(navigation_type.eq(SongSelected)).limit(1)
            .get_result::<BodyTable>(&mut get_connection()).ok().map(|body_table| {
            let body = Body::from_body_table(&body_table, &window_title, &scrolled_window, history.clone(),
                &now_playing, &back_button, &window);
            body.scroll_adjustment.set(body_table.scroll_adjustment);
            Rc::new(body)
        });
        let menu = gtk_box(Vertical);
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text("Menu")
            .popover(&Popover::builder().child(&menu).build()).build();
        header_bar.pack_end(&menu_button);
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
            Body::from_body_table(&body_table, &window_title, &scrolled_window, history.clone(), &now_playing,
                &back_button, &window,
            ).put_to_history(body_table.scroll_adjustment, history.clone());
        }
        if empty_history {
            Rc::new(Body::artists(&window_title, &scrolled_window, history.clone(), &now_playing,
                &Some(back_button.clone()))
            ).set(&window_title, &scrolled_window, history.clone(), &None);
        } else if let Some((body, _)) = history.borrow().last() {
            body.set_window_title(&window_title);
            let Body { widget, scroll_adjustment: body_scroll_adjustment, .. } = body.deref();
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
                Propagation::Proceed
            }
        });
        window.present();
    });
    Ok(application.run())
}
