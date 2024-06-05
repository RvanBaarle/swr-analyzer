use gtk4::gdk::RGBA;
use gtk4::glib;
use relm4::binding::Binding;

glib::wrapper! {
    #[doc = "A data binding storing a value of type [`RGBA`]"]
    pub struct RGBABinding(ObjectSubclass<impl_rgba::RGBABinding>);
}

impl RGBABinding {
    pub fn new<T: Into<RGBA>>(value: T) -> Self {
        let this: Self = glib::Object::new();
        this.set_value(value.into());
        this
    }
}

impl Default for RGBABinding {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Binding for RGBABinding {
    type Target = RGBA;

    fn get(&self) -> Self::Target {
        self.value()
    }

    fn set(&self, value: Self::Target) {
        self.set_value(value)
    }
}

mod impl_rgba {
    use std::cell::RefCell;

    use glib::prelude::*;
    use gtk4::gdk::RGBA;
    use gtk4::glib;
    use gtk4::glib::{ParamSpec, Properties, Value};
    use gtk4::subclass::prelude::*;

    #[derive(Properties, Debug)]
    #[properties(wrapper_type = super::RGBABinding)]
    pub struct RGBABinding {
        #[property(get, set)]
        value: RefCell<RGBA>
    }

    impl Default for RGBABinding {
        fn default() -> Self {
            Self {
                value: RefCell::new(RGBA::new(1.0, 1.0, 1.0, 1.0))
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RGBABinding {
        const NAME: &'static str = "RGBABinding";
        type Type = super::RGBABinding;
    }

    impl ObjectImpl for RGBABinding {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }
        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }
}