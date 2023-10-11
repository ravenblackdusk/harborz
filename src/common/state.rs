use adw::{ApplicationWindow, NavigationView};
use crate::common::window_action::WindowActions;

pub struct State {
    pub window: ApplicationWindow,
    pub body: gtk::Box,
    pub window_actions: WindowActions,
    pub navigation_view: NavigationView,
}
