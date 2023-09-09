use std::cell::RefCell;
use std::env::current_dir;
use std::ops::Deref;
use std::rc::Rc;
use adw::{Application, ApplicationWindow, HeaderBar, WindowTitle};
use adw::gdk::Display;
use adw::glib::{ExitCode, Propagation};
use adw::prelude::*;
use diesel::{delete, ExpressionMethods, insert_into, QueryDsl, RunQueryDsl, update};
use diesel::migration::Result;
use diesel_migrations::MigrationHarness;
use gtk::{Button, CssProvider, IconTheme, MenuButton, Popover, ScrolledWindow, style_context_add_provider_for_display, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk::Align::Fill;
use gtk::Orientation::Vertical;
use log::info;
use db::MIGRATIONS;
use crate::body::{Body, BodyTable, NavigationType::*};
use crate::body::artists::artists;
use crate::common::AdjustableScrolledWindow;
use crate::common::constant::{APP_ID, BACK_ICON};
use crate::common::state::State;
use crate::common::window_action::WindowActions;
use crate::config::Config;
use crate::db::get_connection;
use crate::now_playing::playbin::{PLAYBIN, Playbin};
use crate::schema::bodies::{body_type, navigation_type, params, scroll_adjustment};
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
        let config = config_table.get_result::<Config>(&mut get_connection()).unwrap();
        let header_body = gtk::Box::builder().orientation(Vertical).valign(Fill).build();
        let history_bodies = bodies.filter(navigation_type.eq(History)).get_results::<BodyTable>(&mut get_connection())
            .unwrap();
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
        let window_title = WindowTitle::builder().build();
        let window = ApplicationWindow::builder().application(application).title("Harborz").icon_name("Harborz")
            .content(&header_body).default_width(config.window_width).default_height(config.window_height)
            .maximized(config.maximized == 1).build();
        let window_actions = WindowActions::new(&window);
        let state = Rc::new(State {
            window,
            header_body,
            header_bar: HeaderBar::builder().title_widget(&window_title).build(),
            back_button: Button::builder().icon_name(BACK_ICON).tooltip_text("Home").visible(history_bodies.len() > 1)
                .build(),
            window_actions,
            menu_button: MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text("Menu")
                .popover(&Popover::new()).build(),
            scrolled_window: ScrolledWindow::builder().vexpand(true).build(),
            history: Rc::new(RefCell::new(Vec::new())),
        });
        state.window_actions.change_window_title.action.connect_activate({
            let window_title = window_title.clone();
            move |_, variant| { window_title.set_title(variant.unwrap().str().unwrap()); }
        });
        state.window_actions.change_window_subtitle.action.connect_activate(move |_, variant| {
            window_title.set_subtitle(variant.unwrap().str().unwrap());
        });
        state.header_body.append(&state.header_bar);
        state.header_bar.pack_start(&state.back_button);
        state.header_bar.pack_end(&state.menu_button);
        let body = gtk::Box::builder().orientation(Vertical).build();
        state.header_body.append(&body);
        body.append(&state.scrolled_window);
        let song_selected_body: Rc<RefCell<Option<Rc<Body>>>> = Rc::new(RefCell::new(None));
        let (now_playing_body, bottom_widget, now_playing)
            = now_playing::create(song_selected_body.clone(), state.clone(), &body);
        body.append(&bottom_widget);
        *song_selected_body.borrow_mut() = bodies.filter(navigation_type.eq(SongSelected)).limit(1)
            .get_result::<BodyTable>(&mut get_connection()).ok().map(|BodyTable {
            body_type: body_body_type, params: body_params, scroll_adjustment: body_scroll_adjustment, ..
        }| {
            let body = Body::from_body_table(body_body_type,
                serde_json::from_str::<Vec<Option<String>>>(&body_params).unwrap(), state.clone());
            body.scroll_adjustment.set(body_scroll_adjustment);
            Rc::new(body)
        });
        let empty_history = history_bodies.is_empty();
        for BodyTable {
            body_type: body_body_type, params: body_params, scroll_adjustment: body_scroll_adjustment, ..
        } in history_bodies {
            Body::from_body_table(body_body_type, serde_json::from_str::<Vec<Option<String>>>(&body_params).unwrap(),
                state.clone(),
            ).put_to_history(body_scroll_adjustment, state.history.clone());
        }
        if empty_history {
            Rc::new(artists(state.clone())).set_with_history(state.clone());
        } else if let Some((body, _)) = state.history.borrow().last() {
            body.clone().set(state.clone());
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
                let history = state.history.borrow();
                if let Some((body, _)) = history.last() {
                    body.scroll_adjustment.set(state.scrolled_window.get_adjustment());
                }
                insert_into(bodies).values(
                    history.iter().map(|(body, _)| { (body, History) })
                        .chain(song_selected_body.borrow().iter().map(|body| { (body, SongSelected) }))
                        .map(|(body, body_navigation_type)| {
                            let Body {
                                params: body_params, body_type: body_body_type,
                                scroll_adjustment: body_scroll_adjustment, ..
                            } = body.deref();
                            let value: Vec<Option<String>> = (*body_params).iter()
                                .map(|param| { param.clone().map(|it| { (*it).to_owned() }) }).collect();
                            (params.eq(
                                serde_json::to_string(&value).unwrap()),
                                body_type.eq(body_body_type), scroll_adjustment.eq(body_scroll_adjustment.get()),
                                navigation_type.eq(body_navigation_type)
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
