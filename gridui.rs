#![feature(globs, macro_rules, phase)]

#[phase(plugin, link)]
extern crate log;

extern crate libc;

#[phase(plugin, link)]
extern crate "rust-windows" as windows;

use std::ptr;
use std::cell::RefCell;
use std::comm::channel;

use libc::{c_int};

use windows::main_window_loop;
use windows::ll::types::{UINT, HBRUSH, COLORREF};
use windows::ll::all::{PostQuitMessage, CREATESTRUCT};
use windows::instance::Instance;
use windows::resource::*;
use windows::window::{WindowImpl, Window, WndClass, WindowParams};
use windows::window::{OnCreate, OnSize, OnDestroy, OnPaint, OnFocus, OnEraseBackground};
use windows::window;
use windows::gdi::PaintDc;
use windows::font::Font;
use windows::font;
use windows::font::{Family, Pitch, Quality, CharSet, OutputPrecision, ClipPrecision, FontAttr};

// TODO duplicate of hello.rc
static IDI_ICON: int = 0x101;
static MENU_MAIN: int = 0x201;
//static MENU_NEW: int = 0x202;
//static MENU_EXIT: int = 0x203;

#[deriving(Show)]
enum InputEvent {
    Close,
    MouseDown(u32, u32),
    KeyDown(u32),
}

#[deriving(Copy)]
struct Glyph {
    character: uint,
    background: u32,
    foreground: u32,
}

struct Screen {
    glyphs: Vec<Glyph>,
    width: uint,
}

struct MainFrame {
    win: Window,
    font: RefCell<Option<Font>>,
    screen: Screen,
    input_sink: Sender<InputEvent>,
    grid_height: uint,
}

wnd_proc!(MainFrame, win, WM_CREATE, WM_DESTROY, WM_SIZE, WM_SETFOCUS, WM_PAINT, WM_ERASEBKGND)

impl OnCreate for MainFrame {
    fn on_create(&self, _cs: &CREATESTRUCT) -> bool {
        let font_attr = FontAttr {
            height: self.grid_height as int,
            width: (self.grid_height/2) as int,
            escapement: 0,
            orientation: 0,
            weight: 600, // FW_NORMAL. TODO use FW_DONTCARE (0)?
            italic: false,
            underline: false,
            strike_out: false,
            char_set: CharSet::DEFAULT_CHARSET,
            output_precision: OutputPrecision::OUT_DEFAULT_PRECIS,
            clip_precision: ClipPrecision::CLIP_DEFAULT_PRECIS,
            quality: Quality::ANTIALIASED_QUALITY,
            pitch: Pitch::DEFAULT_PITCH,
            family: Family::FF_DONTCARE,
            face: Some("Courier New".to_string()),
        };
        let font = font::Font::new(&font_attr);
        debug!("font: {}", font); // the trait `core::fmt::Show` is not implemented for the type `rust-windows::font::Font`
        match font {
            None => false,
            Some(f) => {
                *self.font.borrow_mut() = Some(f);
                true
            }
        }
    }
}

impl OnSize for MainFrame {
    fn on_size(&self, width: int, height: int) {
        // SWP_NOOWNERZORDER | SWP_NOZORDER
        //let h = self.text_height;
        //self.edit.borrow().expect("edit is empty")
        //    .set_window_pos(0, h, width, height - h, 0x200 | 0x4);
    }
}

impl OnDestroy for MainFrame {
    fn on_destroy(&self) {
        unsafe {
            PostQuitMessage(0 as c_int);
        }
        self.input_sink.send(InputEvent::Close);
    }
}

impl OnPaint for MainFrame {
    fn on_paint(&self) {
        let font = self.font.borrow();
        let pdc = PaintDc::new(self).expect("Paint DC");
        pdc.dc.select_font(&font.expect("font is empty"));
        
        let grid_width = self.grid_height/2;
        for (row_idx, row) in self.screen.glyphs.chunks(self.screen.width).enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                pdc.dc.set_text_color(cell.foreground as COLORREF);
                pdc.dc.set_background_color(cell.background as COLORREF);
                pdc.dc.text_out((col_idx*grid_width) as int, (row_idx*self.grid_height) as int, "0");
            }
        }
    }
}

impl OnFocus for MainFrame {
    fn on_focus(&self, _w: Window) {
        //self.edit.borrow().expect("edit is empty").set_focus();
    }
}

impl OnEraseBackground for MainFrame {
    fn on_erase_background(&self) -> bool {
        true
    }
}

impl MainFrame {
    fn new(instance: Instance, title: String, input_sink: Sender<InputEvent>) -> Option<Window> {
        let icon = Image::load_resource(instance, IDI_ICON, ImageType::IMAGE_ICON, 0, 0);
        let wnd_class = WndClass {
            classname: "MainFrame".to_string(),
            style: 0x0001 | 0x0002, // CS_HREDRAW | CS_VREDRAW
            icon: icon,
            icon_small: None,
            cursor: None, //Image::load_cursor_resource(32514), // hourglass
            background: (5i + 1) as HBRUSH,
            menu: MenuResource::MenuId(MENU_MAIN),
            cls_extra: 0,
            wnd_extra: 0,
        };
        let res = wnd_class.register(instance);
        if !res {
            return None;
        }

        let wproc = box MainFrame {
            win: Window::null(),
            font: RefCell::new(None),
            input_sink: input_sink,
            screen: Screen{
              width:20,
              glyphs: Vec::from_fn(20*6, |_| { Glyph{character:0, foreground:0xff5555, background: 0x000000}})
            },
            grid_height: 30,
        };

        let win_params = WindowParams {
            window_name: title,
            style: window::WS_OVERLAPPEDWINDOW,
            x: 0,
            y: 0,
            width: 400,
            height: 400,
            parent: Window::null(),
            menu: ptr::null_mut(),
            ex_style: 0,
        };

        Window::new(instance, Some(wproc as Box<WindowImpl+Send>),
                    wnd_class.classname.as_slice(), &win_params)
    }
}

fn main() {
    window::init_window_map();

    let (tx, rx) = channel();
    
    spawn(move|| {
        println!("Received {}", rx.recv())
    });
    
    let instance = Instance::main_instance();
    let main = MainFrame::new(instance, "Grid UI".to_string(), tx);
    let main = main.unwrap();

    main.show(1);
    main.update();

    let exit_code = main_window_loop();
    std::os::set_exit_status(exit_code as int);
}
