use std::cell::RefCell;
use std::ops;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::translate::*;
use glib::ToValue;

use libc::c_char;

use crate::nameable::*;

pub mod ffi {
    pub type Foo = <super::Foo as super::ObjectSubclass>::Instance;
}

// Class struct aka "vtable"
//
// Here we would store virtual methods and similar
#[repr(C)]
pub struct FooClass {
    pub parent_class: glib::gobject_ffi::GObjectClass,
    pub increment: Option<unsafe extern "C" fn(*mut ffi::Foo, inc: i32) -> i32>,
    pub incremented: Option<unsafe extern "C" fn(*mut ffi::Foo, val: i32, inc: i32)>,
}

unsafe impl ClassStruct for FooClass {
    type Type = Foo;
}

impl ops::Deref for FooClass {
    type Target = glib::Class<glib::Object>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const _ as *const Self::Target) }
    }
}

impl ops::DerefMut for FooClass {
    fn deref_mut(&mut self) -> &mut glib::Class<glib::Object> {
        unsafe { &mut *(self as *mut _ as *mut glib::Class<glib::Object>) }
    }
}

// Our private state for the class
//
// We use RefCells here for each field as GObject conceptually uses interior mutability everywhere.
// If this was to be used from multiple threads, these would have to be mutexes or otherwise
// Sync+Send
#[derive(Debug, Default)]
pub struct Foo {
    name: RefCell<Option<String>>,
    counter: RefCell<i32>,
}

#[glib::object_subclass]
impl ObjectSubclass for Foo {
    const NAME: &'static str = "ExFoo";
    type ParentType = glib::Object;
    type Type = super::Foo;
    type Class = FooClass;
    type Interfaces = (Nameable,);

    fn class_init(klass: &mut Self::Class) {
        klass.increment = Some(increment_default_trampoline);
        klass.incremented = Some(incremented_default_trampoline);
    }
}

impl ObjectImpl for Foo {
    fn signals() -> &'static [glib::subclass::Signal] {
        use once_cell::sync::Lazy;
        static SIGNALS: Lazy<Vec<glib::subclass::Signal>> = Lazy::new(|| {
            vec![glib::subclass::Signal::builder(
                "incremented",
                &[i32::static_type().into(), i32::static_type().into()],
                glib::Type::UNIT.into(),
            )
            .class_handler(|_, args| {
                let obj = args[0].get::<glib::Object>().unwrap().unwrap();
                let val = args[1].get::<i32>().unwrap().unwrap();
                let inc = args[2].get::<i32>().unwrap().unwrap();

                unsafe {
                    let klass = &*(obj.get_object_class() as *const _ as *const FooClass);
                    if let Some(ref func) = klass.incremented {
                        func(obj.as_ptr() as *mut ffi::Foo, val, inc);
                    }
                }

                None
            })
            .build()]
        });

        SIGNALS.as_ref()
    }

    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::string(
                "name",
                "Name",
                "Name of this object",
                None,
                glib::ParamFlags::READWRITE,
            )]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(
        &self,
        obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
        match pspec.get_name() {
            "name" => {
                let name = value.get().unwrap();
                self.set_name(obj, name);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.get_name() {
            "name" => self.get_name(obj).to_value(),
            _ => unimplemented!(),
        }
    }
}

impl NameableImpl for Foo {
    fn get_name(&self, nameable: &Self::Type) -> Option<String> {
        self.get_name(nameable.dynamic_cast_ref().unwrap())
    }
}

impl Foo {
    //
    // Safe implementations. These take the wrapper type, and not &Self, as first argument
    //
    fn increment(&self, this: &super::Foo, inc: i32) -> i32 {
        let mut val = self.counter.borrow_mut();

        *val += inc;

        this.emit_by_name("incremented", &[&*val, &inc]).unwrap();

        *val
    }

    fn incremented(&self, _this: &super::Foo, _val: i32, _inc: i32) {
        // Could do something here. Default/class handler of the "incremented"
        // signal that could be overriden by subclasses
    }

    fn get_counter(&self, _this: &super::Foo) -> i32 {
        *self.counter.borrow()
    }

    fn get_name(&self, _this: &super::Foo) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, _this: &super::Foo, name: Option<String>) {
        *self.name.borrow_mut() = name;
    }
}

//
// Public C functions below
//

// Virtual method callers
/// # Safety
///
/// Must be a FooInstance object.
#[no_mangle]
pub unsafe extern "C" fn ex_foo_increment(this: *mut ffi::Foo, inc: i32) -> i32 {
    let klass = (*this).get_class();

    (klass.increment.unwrap())(this, inc)
}

// Trampolines to safe Rust implementations
/// # Safety
///
/// Must be a FooInstance object.
#[no_mangle]
pub unsafe extern "C" fn ex_foo_get_counter(this: *mut ffi::Foo) -> i32 {
    let imp = (*this).get_impl();
    imp.get_counter(&from_glib_borrow(this))
}

/// # Safety
///
/// Must be a FooInstance object.
#[no_mangle]
pub unsafe extern "C" fn ex_foo_get_name(this: *mut ffi::Foo) -> *mut c_char {
    let imp = (*this).get_impl();
    imp.get_name(&from_glib_borrow(this)).to_glib_full()
}

// GObject glue
/// # Safety
///
/// Must be a valid C string, 0-terminated.
#[no_mangle]
pub unsafe extern "C" fn ex_foo_new(name: *const c_char) -> *mut ffi::Foo {
    let obj = glib::Object::new::<super::Foo>(&[("name", &*glib::GString::from_glib_borrow(name))])
        .unwrap();
    obj.to_glib_full()
}

#[no_mangle]
pub extern "C" fn ex_foo_get_type() -> glib::ffi::GType {
    Foo::get_type().to_glib()
}

// Virtual method default implementation trampolines
unsafe extern "C" fn increment_default_trampoline(this: *mut ffi::Foo, inc: i32) -> i32 {
    let imp = (*this).get_impl();
    imp.increment(&from_glib_borrow(this), inc)
}

unsafe extern "C" fn incremented_default_trampoline(this: *mut ffi::Foo, val: i32, inc: i32) {
    let imp = (*this).get_impl();
    imp.incremented(&from_glib_borrow(this), val, inc);
}
