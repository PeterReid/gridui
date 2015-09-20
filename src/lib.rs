
#![crate_type = "lib"]
#![crate_name = "gridui"]

//#[phase(plugin, link)]
//extern crate log;

#![feature(libc, borrow_state)]

extern crate libc;

extern crate winapi;

#[macro_use]
extern crate rust_windows as windows;
extern crate gdi32 as gdi32;
extern crate kernel32 as kernel32;
extern crate user32 as user32;
extern crate unicode_segmentation;

pub mod gridui;
pub mod glyphcode;

