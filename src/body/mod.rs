use std::rc::Rc;
use std::sync::Arc;
use adw::{HeaderBar, NavigationPage, WindowTitle};
use adw::gio::{SimpleAction, SimpleActionGroup};
use adw::prelude::*;
use gtk::{Image, Label, MenuButton, Popover, ScrolledWindow, Widget};
use gtk::Orientation::Vertical;
use crate::common::state::State;

pub mod collection;
mod merge;
pub mod artists;
pub mod download;

#[derive(diesel::Queryable, diesel::Selectable, Debug)]
#[diesel(table_name = crate::schema::bodies)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BodyTable {
    pub id: i32,
    pub body_type: BodyType,
    pub scroll_adjustment: Option<f32>,
    pub params: String,
}

#[derive(Debug, diesel_derive_enum::DbEnum)]
pub enum BodyType {
    Artists,
    Albums,
    Songs,
    Collections,
}

fn next_icon() -> Image {
    Image::builder().icon_name("go-next-symbolic").margin_start(10).margin_end(8).build()
}

trait Castable {
    fn first_child(self) -> Option<Widget>;
    fn set_label(self, label: &str) -> Label;
}

impl Castable for Option<Widget> {
    fn first_child(self) -> Option<Widget> {
        self.and_downcast::<gtk::Box>().unwrap().first_child()
    }
    fn set_label(self, label: &str) -> Label {
        let result = self.and_downcast::<Label>().unwrap();
        result.set_label(label);
        result
    }
}

const ARTIST: &'static str = "Artist";
const ALBUM: &'static str = "Album";
const SONG: &'static str = "Song";
const RERENDER: &'static str = "rerender";
pub const PARAMS: &'static str = "params";
pub const BODY_TYPE: &'static str = "body_type";

struct Body {
    action_group: SimpleActionGroup,
    rerender: SimpleAction,
    menu_button: MenuButton,
    window_title: WindowTitle,
    header_bar: HeaderBar,
    scrolled_window: ScrolledWindow,
    popover_box: gtk::Box,
    navigation_page: NavigationPage,
}

impl Body {
    fn new(title: &str, state: Rc<State>, tag: Option<&str>, params: Vec<Option<Arc<String>>>, body_type: BodyType)
        -> Self {
        let action_group = SimpleActionGroup::new();
        let rerender = SimpleAction::new(RERENDER, None);
        action_group.add_action(&rerender);
        let popover_box = gtk::Box::builder().orientation(Vertical).build();
        let menu_button = MenuButton::builder().icon_name("open-menu-symbolic").tooltip_text("Menu")
            .popover(&Popover::builder().child(&popover_box).build()).build();
        popover_box.append(&collection::button::create(state, &menu_button));
        let child = gtk::Box::builder().orientation(Vertical).build();
        let window_title = WindowTitle::builder().title(title).build();
        let header_bar = HeaderBar::builder().title_widget(&window_title).build();
        child.append(&header_bar);
        let scrolled_window = ScrolledWindow::builder().vexpand(true).build();
        child.append(&scrolled_window);
        header_bar.pack_end(&menu_button);
        let pop_down = SimpleAction::new(POP_DOWN, None);
        action_group.add_action(&pop_down);
        pop_down.connect_activate({
            let menu_button = menu_button.clone();
            move |_, _| { menu_button.popdown(); }
        });
        let change_title = SimpleAction::new(CHANGE_TITLE, Some(&String::static_variant_type()));
        action_group.add_action(&change_title);
        change_title.connect_activate({
            let window_title = window_title.clone();
            move |_, entity| { window_title.set_title(entity.unwrap().str().unwrap()); }
        });
        let change_subtitle = SimpleAction::new(CHANGE_SUBTITLE, Some(&String::static_variant_type()));
        action_group.add_action(&change_subtitle);
        change_subtitle.connect_activate({
            let window_title = window_title.clone();
            move |_, entity| { window_title.set_subtitle(entity.unwrap().str().unwrap()); }
        });
        let navigation_page = create_navigation_page(&child, tag.unwrap_or(title), params, body_type);
        navigation_page.insert_action_group(NAVIGATION_PAGE, Some(&action_group));
        Self {
            action_group,
            rerender,
            menu_button,
            window_title,
            header_bar,
            scrolled_window,
            popover_box,
            navigation_page,
        }
    }
}

const POP_DOWN: &'static str = "pop_down";
const CHANGE_TITLE: &'static str = "change_title";
const CHANGE_SUBTITLE: &'static str = "change_subtitle";

fn create_navigation_page(child: &gtk::Box, tag: &str, params: Vec<Option<Arc<String>>>, body_type: BodyType)
    -> NavigationPage {
    let navigation_page = NavigationPage::builder().child(child).title(tag).tag(tag).build();
    unsafe {
        navigation_page.set_data(PARAMS, params);
        navigation_page.set_data(BODY_TYPE, body_type);
    }
    navigation_page
}

const NAVIGATION_PAGE: &'static str = "navigation_page";
const START_MERGE: &'static str = "start_merge";
const HEADER_BAR_START_MERGE: &'static str = "header_bar_start_merge";

fn action_name(name: &str) -> String {
    format!("{NAVIGATION_PAGE}.{name}")
}

fn handle_render<R: Fn() + 'static>(render: R, rerender: SimpleAction) {
    render();
    rerender.connect_activate(move |_, _| { render(); });
}
