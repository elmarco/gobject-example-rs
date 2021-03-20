use libc::c_char;

#[repr(C)]
pub struct ExFoo {
    pub parent: glib::gobject_ffi::GObject,
}

#[repr(C)]
pub struct ExFooClass {
    pub parent_class: glib::gobject_ffi::GObjectClass,
    pub increment: Option<unsafe extern "C" fn(*mut ExFoo, inc: i32) -> i32>,
    pub incremented: Option<unsafe extern "C" fn(*mut ExFoo, val: i32, inc: i32)>,
}

extern "C" {
    pub fn ex_foo_new(name: *const c_char) -> *mut ExFoo;
    pub fn ex_foo_get_type() -> glib::ffi::GType;

    pub fn ex_foo_increment(this: *mut ExFoo, inc: i32) -> i32;
    pub fn ex_foo_get_counter(this: *mut ExFoo) -> i32;
    pub fn ex_foo_get_name(this: *mut ExFoo) -> *mut c_char;
}
