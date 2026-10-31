//! A box that reports its allocated width. GTK 4 has no size-allocate signal, so a
//! view that needs its own width (the media progress line) is laid out in one of these.
//! It costs nothing while idle: it only runs when GTK lays the bar out anyway.

use std::cell::{Cell, RefCell};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

type OnWidth = Box<dyn Fn(u32)>;

glib::wrapper! {
    pub struct WidthBox(ObjectSubclass<imp::WidthBox>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl WidthBox {
    pub fn new(orientation: gtk4::Orientation) -> Self {
        let this: Self = glib::Object::new();
        this.set_orientation(orientation);
        this
    }

    /// Call `f` with the width in pixels whenever it changes. Called from an idle
    /// callback, never during layout, so `f` may change widgets freely.
    pub fn connect_width(&self, f: impl Fn(u32) + 'static) {
        *self.imp().on_width.borrow_mut() = Some(Box::new(f));
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct WidthBox {
        pub(super) on_width: RefCell<Option<OnWidth>>,
        width: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WidthBox {
        const NAME: &'static str = "ZbWidthBox";
        type Type = super::WidthBox;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for WidthBox {}
    impl BoxImpl for WidthBox {}

    impl WidgetImpl for WidthBox {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            if self.width.replace(width) == width {
                return;
            }
            let weak = self.obj().downgrade();
            glib::idle_add_local_once(move || {
                if let Some(this) = weak.upgrade()
                    && let Some(f) = this.imp().on_width.borrow().as_ref()
                {
                    f(this.imp().width.get().max(0) as u32);
                }
            });
        }
    }
}
