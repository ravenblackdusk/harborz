use std::cell::RefCell;
use adw::glib::{self, ParamSpec, Properties};
use adw::glib::subclass::Signal;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::*;
use gtk::Widget;
use once_cell::sync::Lazy;
use crate::common::wrapper::STREAM_STARTED;

#[derive(Debug, Default, Properties)]
#[properties(wrapper_type = super::Wrapper)]
pub struct Wrapper {
    #[property(get, set = Self::set_child)]
    child: RefCell<Option<Widget>>,
}

impl Wrapper {
    fn set_child(&self, child: Option<Widget>) {
        if let Some(child) = child {
            child.set_parent(self.obj().as_ref());
            *self.child.borrow_mut() = Some(child);
        }
    }
}

//noinspection RsTraitImplementation
#[glib::object_subclass]
impl ObjectSubclass for Wrapper {
    const NAME: &'static str = "Wrapper";
    type Type = super::Wrapper;
    type ParentType = Widget;
    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
    }
}

static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
    vec![Signal::builder(STREAM_STARTED).param_types([i32::static_type()]).build()]
});

impl ObjectImpl for Wrapper {
    fn properties() -> &'static [ParamSpec] {
        Self::derived_properties()
    }
    fn signals() -> &'static [Signal] {
        SIGNALS.as_ref()
    }
    fn set_property(&self, id: usize, value: &glib::Value, param_spec: &ParamSpec) {
        self.derived_set_property(id, value, param_spec)
    }
    fn property(&self, id: usize, param_spec: &ParamSpec) -> glib::Value {
        self.derived_property(id, param_spec)
    }
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn dispose(&self) {
        if let Some(child) = self.child.borrow_mut().take() {
            child.unparent();
        }
    }
}

impl WidgetImpl for Wrapper {}
