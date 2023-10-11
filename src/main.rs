use std::cell::RefCell;
use std::env::current_dir;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use adw::{Application, ApplicationWindow, NavigationPage, NavigationView};
use adw::gdk::Display;
use adw::glib::{ExitCode, Propagation, SignalHandlerId, timeout_add_local_once};
use adw::glib::translate::FromGlib;
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_into, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{CssProvider, IconTheme, ScrolledWindow, style_context_add_provider_for_display, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::Align::Fill;
use gtk::Orientation::Vertical;
use log::info;
use db::MIGRATIONS;
use crate::body::{BodyTable, BodyType, BODY_TYPE, PARAMS};
use crate::body::artists::artists_page;
use crate::body::collection::page::{COLLECTION, collection_page};
use crate::body::download::albums::albums_page;
use crate::body::download::songs::songs_page;
use crate::common::constant::APP_ID;
use crate::common::state::State;
use crate::common::window_action::WindowActions;
use crate::config::Config;
use crate::db::get_connection;
use crate::now_playing::playbin::{PLAYBIN, Playbin};
use crate::schema::bodies::{body_type, params, scroll_adjustment};
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

fn handle_scroll(scroll: Option<f64>, navigation_page: &NavigationPage) {
    let signal_handler_id = Rc::new(RefCell::new(None::<SignalHandlerId>));
    *signal_handler_id.borrow_mut() = Some(navigation_page.connect_realize({
        let signal_handler_id = signal_handler_id.clone();
        move |navigation_page| {
            if let Some(scroll) = scroll {
                timeout_add_local_once(Duration::from_millis(50), {
                    let navigation_page = navigation_page.clone();
                    move || {
                        navigation_page.child().unwrap().last_child().and_downcast::<ScrolledWindow>().unwrap()
                            .vadjustment().set_value(scroll);
                    }
                });
            }
            navigation_page.disconnect(unsafe {
                SignalHandlerId::from_glib(signal_handler_id.borrow().as_ref().unwrap().as_raw())
            });
        }
    }));
}

fn main() -> Result<ExitCode> {
    std_logger::Config::logfmt().init();
    gstreamer::init()?;
    get_connection().run_pending_migrations(MIGRATIONS)?;
    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(|application| {
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let body = gtk::Box::builder().orientation(Vertical).valign(Fill).build();
        let history_bodies = bodies.get_results::<BodyTable>(&mut get_connection()).unwrap();
        let css_provider = CssProvider::new();
        css_provider.load_from_data("#accent-bg { background-color: @accent_bg_color; } \
        #dialog-bg { background-color: @dialog_bg_color; }
        #small-slider slider { min-width: 16px; min-height: 16px; } trough { min-height: 4px; } \
        #insensitive-fg { color: alpha(@window_fg_color, 0.5); }");
        let display = Display::default().unwrap();
        style_context_add_provider_for_display(&display, &css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);
        let working_dir = current_dir().unwrap();
        info!("working directory is [{}]", working_dir.to_str().unwrap());
        IconTheme::for_display(&display).add_search_path(working_dir.join("icons"));
        let window = ApplicationWindow::builder().application(application).title("Harborz").icon_name("Harborz")
            .content(&body).default_width(config.window_width).default_height(config.window_height)
            .maximized(config.maximized == 1).build();
        let window_actions = WindowActions::new(&window);
        let state = Rc::new(State {
            window,
            body: body.clone(),
            window_actions,
            navigation_view: NavigationView::new(),
        });
        state.body.append(&state.navigation_view);
        let (now_playing_body, bottom_widget, now_playing) = now_playing::create(state.clone());
        state.body.append(&bottom_widget);
        let artists_page = artists_page(state.clone());
        state.navigation_view.add(&artists_page);
        let collection_page = collection_page(state.clone());
        state.navigation_view.add(&collection_page);
        for body_table in history_bodies {
            let body_params = serde_json::from_str::<Vec<Option<String>>>(&body_table.params).unwrap().into_iter()
                .map(|it| { it.map(Arc::new) }).collect();
            let scroll = body_table.scroll_adjustment.map(|it| { it as f64 });
            match body_table.body_type {
                BodyType::Artists => { handle_scroll(scroll, &artists_page); }
                BodyType::Albums => { state.navigation_view.push(&albums_page(body_params, state.clone(), scroll)); }
                BodyType::Songs => { state.navigation_view.push(&songs_page(body_params, state.clone(), scroll)); }
                BodyType::Collections => {
                    state.navigation_view.push_by_tag(COLLECTION);
                    handle_scroll(scroll, &collection_page);
                }
            }
        }
        if config.now_playing_body_realized == 1 {
            now_playing.borrow().realize_body(state.clone(), &now_playing_body);
        }
        state.window.connect_close_request({
            let state = state.clone();
            move |window| {
                let (width, height) = window.default_size();
                update(config_table).set((window_width.eq(width), window_height.eq(height),
                    maximized.eq(if window.is_maximized() { 1 } else { 0 }),
                    current_song_position.eq(PLAYBIN.get_position().unwrap_or(0) as i64)
                )).execute(&mut get_connection()).unwrap();
                delete(bodies).execute(&mut get_connection()).unwrap();
                insert_into(bodies).values(
                    state.navigation_view.navigation_stack().iter::<NavigationPage>().map(|navigation_page| {
                        let navigation_page = navigation_page.unwrap();
                        let (params_ref, body_type_ref) = unsafe {
                            (navigation_page.data::<Vec<Option<Arc<String>>>>(PARAMS).unwrap().as_ref(),
                                navigation_page.data::<BodyType>(BODY_TYPE).unwrap().as_ref())
                        };
                        (params.eq(serde_json::to_string(&params_ref.iter().map(Option::as_deref).collect::<Vec<_>>())
                            .unwrap()),
                            body_type.eq(body_type_ref),
                            scroll_adjustment.eq(Some(navigation_page.child().unwrap().last_child()
                                .and_downcast::<ScrolledWindow>().unwrap().vadjustment().value() as f32)),
                        )
                    }).collect::<Vec<_>>()
                ).execute(&mut get_connection()).unwrap();
                Propagation::Proceed
            }
        });
        state.window.present();
    });
    Ok(application.run())
}
