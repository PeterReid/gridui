
#![crate_type = "lib"]
#![crate_name = "gridui"]

//#[phase(plugin, link)]
//extern crate log;

#![feature(collections, libc, std_misc)]

extern crate libc;

extern crate winapi;

#[macro_use]
extern crate "rust-windows" as windows;
extern crate "gdi32-sys" as gdi32;
extern crate "kernel32-sys" as kernel32;
extern crate "user32-sys" as user32;

pub mod gridui;
pub mod glyphcode;

