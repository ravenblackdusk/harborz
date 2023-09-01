use adw::ApplicationWindow;
use adw::gio::SimpleAction;
use adw::glib::StaticVariantType;
use adw::prelude::*;

pub struct WindowAction {
    pub action: SimpleAction,
}

impl WindowAction {
    fn new<V: StaticVariantType>(name: &'static str, application_window: &ApplicationWindow) -> Self {
        let action = SimpleAction::new(name, Some(&V::static_variant_type()));
        application_window.add_action(&action);
        Self { action }
    }
    pub fn activate(&self, variant: impl ToVariant) {
        self.action.activate(Some(&variant.to_variant()));
    }
}

pub struct WindowActions {
    pub song_selected: WindowAction,
    pub stream_started: WindowAction,
    pub change_window_title: WindowAction,
    pub change_window_subtitle: WindowAction,
}

impl WindowActions {
    pub fn new(application_window: &ApplicationWindow) -> Self {
        Self {
            song_selected: WindowAction::new::<String>("song-selected", application_window),
            stream_started: WindowAction::new::<i32>("stream-started", application_window),
            change_window_title: WindowAction::new::<String>("change-window-title", application_window),
            change_window_subtitle: WindowAction::new::<String>("change-window-subtitle", application_window),
        }
    }
}
