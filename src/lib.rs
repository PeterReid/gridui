
#![crate_type = "lib"]
#![crate_name = "gridui"]

//#[phase(plugin, link)]
//extern crate log;

//#![feature(libc, borrow_state)]

extern crate libc;
extern crate unicode_segmentation;

#[cfg(target_os="windows")]
extern crate winapi;

#[macro_use]
#[cfg(target_os="windows")]
extern crate rust_windows as windows;
#[cfg(target_os="windows")]
extern crate gdi32 as gdi32;
#[cfg(target_os="windows")]
extern crate kernel32 as kernel32;
#[cfg(target_os="windows")]
extern crate user32 as user32;


#[cfg(target_os="windows")]
pub mod gridui;

#[cfg(target_os="linux")]
pub mod x11;

#[cfg(target_os="linux")]
pub use x11 as gridui;

#[cfg(target_os="linux")]
extern crate x11_dl;

pub mod glyphcode;

pub mod screen;
pub mod input_event;
pub mod glyph_parts;
