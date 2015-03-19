/*
    gridui
    Copyright (C) 2014  Peter Reid

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::ptr;
use std::cell::{RefCell};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::Thread;

use std::ops::{Deref, DerefMut};

use libc::{c_int};

use windows::main_window_loop;
use winapi::{UINT, HBRUSH, COLORREF, LPARAM, WPARAM, LRESULT};
use user32::{PostQuitMessage, GetSysColor};
use winapi::{CREATESTRUCTW};
use gdi32::{GetStockObject, SetDCBrushColor};
use windows::instance::Instance;
use windows::resource::*;
use windows::window::{WindowImpl, Window, WndClass, WindowParams};
use windows::window::{OnCreate, OnSize, OnDestroy, OnPaint, OnEraseBackground, OnMessage};
use windows::window::{OnLeftButtonDown, OnLeftButtonUp, OnKeyDown, OnKeyUp};
use windows::window;
use windows::gdi::{PaintDc};
use windows::font::Font;
use windows::font;
use windows::font::{Family, Pitch, Quality, CharSet, OutputPrecision, ClipPrecision, FontAttr};
use glyphcode;

#[derive(Copy, Debug)]
pub enum InputEvent {
    Close,
    MouseDown(u32, u32),
    MouseUp(u32, u32),
    KeyDown(u32),
    KeyUp(u32),
    Size(u32, u32),
}

#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub character: u32,
    pub background: u32,
    pub foreground: u32,
}

#[derive(Clone)]
pub struct Screen {
    pub glyphs: Vec<Glyph>,
    pub width: u32,
}

struct MainFrameState {
    screen: Screen,
    announced_grid_size: (i32, i32),
}

struct MainFrame {
    win: Window,
    font: RefCell<Option<Font>>,
    input_sink: Sender<InputEvent>,
    screen_source: Receiver<Screen>,
    grid_height: u32,
    state: RefCell<MainFrameState>,
}

const WM_CHECK_SCREENS : UINT = 0x0401;

wnd_proc!(MainFrame, win, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_KEYDOWN, WM_KEYUP, WM_SIZE, WM_PAINT, WM_ERASEBKGND, ANY);

impl OnCreate for MainFrame {
    fn on_create(&self, _cs: &CREATESTRUCTW) -> bool {
        let font_attr = FontAttr {
            height: self.grid_height as isize,
            width: (self.grid_height/2) as isize,
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
    fn on_size(&self, width: isize, height: isize) {
        let grid_width = self.grid_height/2;
        let cols = width as u32 / grid_width as u32;
        let rows = height as u32 / self.grid_height as u32;
        
        let size = (cols as i32, rows as i32);
        self.with_state_mut(|state: &mut MainFrameState| {
            if size != state.announced_grid_size {
                state.announced_grid_size = size;
                self.input_sink.send(InputEvent::Size(cols, rows));
            }
        });
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
        //note: VirtualAlloc the buffer!
        
        let font = self.font.borrow();
        let pdc = PaintDc::new(self).expect("Paint DC");
        pdc.dc.select_font(&font.expect("font is empty"));
        
        self.with_state(|state: & MainFrameState| {
            let ref screen = state.screen;
            let grid_width = self.grid_height/2;
            if state.screen.width > 0 {
                for (row_idx, row) in screen.glyphs.chunks(screen.width as usize).enumerate() {
                    for (col_idx, cell) in row.iter().enumerate() {
                        pdc.dc.set_text_color(cell.foreground as COLORREF);
                        pdc.dc.set_background_color(cell.background as COLORREF);
                        
                        let mut char_str = String::new();
                        char_str.push(glyphcode::as_char(cell.character).unwrap_or(' '));
                        pdc.dc.text_out((col_idx*grid_width as usize) as isize, (row_idx*(self.grid_height as usize)) as isize, &char_str[]);
                    }
                }
            }
            
            if let Some(client_rect) = self.win.client_rect(){
                let max_filled_x = screen.width * grid_width;
                let max_filled_y = if screen.width>0 { (screen.glyphs.len() as u32 / screen.width) * self.grid_height } else { 0 };
                
                let filler_color = unsafe { GetSysColor(15 /* COLOR_3DFACE */) as COLORREF };
                unsafe { SetDCBrushColor(pdc.dc.raw, filler_color) };
                let null_pen = unsafe { GetStockObject(8 /* NULL_PEN */) };
                let dc_brush = unsafe { GetStockObject(18 /* DC_BRUSH */) };
                
                pdc.dc.select_object(null_pen);
                pdc.dc.select_object(dc_brush);
                pdc.dc.rect((max_filled_x as isize + 1, 0), (client_rect.right as isize + 1, client_rect.bottom as isize + 1));
                pdc.dc.rect((0, max_filled_y as isize), (max_filled_x as isize + 2, client_rect.bottom as isize + 1));
            }
        });
        
    }
}

impl OnEraseBackground for MainFrame {
    fn on_erase_background(&self) -> bool {
        true
    }
}

impl OnLeftButtonDown for MainFrame {
    fn on_left_button_down(&self, x: isize, y: isize, _flags: u32) {
        let grid_width = self.grid_height/2;
        let col = (x as u32) / (grid_width as u32);
        let row = (y as u32) / (self.grid_height as u32);
        self.input_sink.send(InputEvent::MouseDown(col as u32, row as u32));
    }
}

impl OnLeftButtonUp for MainFrame {
    fn on_left_button_up(&self, x: isize, y: isize, _flags: u32) {
        let grid_width = self.grid_height/2;
        let col = (x as u32) / (grid_width as u32);
        let row = (y as u32) / (self.grid_height as u32);
        self.input_sink.send(InputEvent::MouseUp(col as u32, row as u32));
    }
}

fn windows_keycode_to_character(keycode: u8) -> Option<u32> {
    if ('A' as u8) <= keycode && keycode <= ('Z' as u8) {
        return Some(0x1000 + (((keycode - ('A' as u8)) as u32) << 4));
    } else if ('0' as u8) <= keycode && keycode <= ('9' as u8) {
        return Some(10 + (keycode - ('0' as u8)) as u32);
    } else if (' ' as u8) == keycode {
        return Some(0);
    }
    return None;
}

impl OnKeyDown for MainFrame {
    fn on_key_down(&self, keycode: u8, flags: u32) -> bool {
        if let Some(character) = windows_keycode_to_character(keycode) {
            self.input_sink.send(InputEvent::KeyDown(character));
        } else {
            println!("Unknown key down {} {}", keycode, flags);
        }
        
        return true;
    }
}

impl OnKeyUp for MainFrame {
    fn on_key_up(&self, keycode: u8, flags: u32) -> bool {
        if let Some(character) = windows_keycode_to_character(keycode) {
            self.input_sink.send(InputEvent::KeyUp(character));
        } else {
            println!("Unknown key up {} {}", keycode, flags);
        }
        return true;
    }
}

impl OnMessage for MainFrame {
    fn on_message(&self, msg: UINT, _: WPARAM, _: LPARAM) -> Option<LRESULT> {
        if msg==WM_CHECK_SCREENS {
            self.check_for_new_screen();
            return Some(0);
        }
        None
    }
}
impl MainFrame {
    fn new(instance: Instance, title: String, input_sink: Sender<InputEvent>, screen_source: Receiver<Screen>) -> Option<Window> {
        let wnd_class = WndClass {
            classname: "MainFrame".to_string(),
            style: 0x0001 | 0x0002, // CS_HREDRAW | CS_VREDRAW
            icon: None,
            icon_small: None,
            cursor: Image::load_cursor_resource(32512), // standard arrow
            background: (5u32 + 1) as HBRUSH,
            menu: MenuResource::MenuId(0),
            cls_extra: 0,
            wnd_extra: 0,
        };
        let res = wnd_class.register(instance);
        if !res {
            return None;
        }

        let wproc = Box::new(MainFrame {
            win: Window::null(),
            font: RefCell::new(None),
            input_sink: input_sink,
            screen_source: screen_source,
            state: RefCell::new(MainFrameState{
                screen: Screen{
                    width:0,
                    glyphs: Vec::new()
                },
                announced_grid_size: (-1,-1),  
            }),
            grid_height: 30,
        });

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

        Window::new(instance, Some(wproc as Box<WindowImpl + 'static>),
                    &wnd_class.classname[], &win_params)
    }
    
    fn with_state_mut<F>(&self, f: F)
        where F: FnOnce(&mut MainFrameState)
    {
        if let Some(mut state) = self.state.try_borrow_mut() {
            f(state.deref_mut());
        }
    }
    
    fn with_state<F>(&self, f: F)
        where F: FnOnce(&MainFrameState)
    {
        if let Some(state) = self.state.try_borrow() {
            f(state.deref());
        }
    }
    
    fn check_for_new_screen(&self) {
        let mut new_screen = None;
        loop {
            match self.screen_source.try_recv() {
                Ok(screen) => {new_screen = Some(screen) },
                Err(_) => { break; }
            }
        }
        
        if let Some(s) = new_screen {
            self.with_state_mut(|state: &mut MainFrameState| {
                state.screen = s.clone();
                self.win.invalidate(false);
            });
        }
    }
}

pub trait GridUiInterface {
    fn send_screen(&self, screen: Screen);
    fn get_input_event(&self) -> InputEvent;
}

pub struct WindowsGridUi {
    screen_sink: Sender<Screen>,
    pub input_event_source: Receiver<InputEvent>,
    window: Window,
}

impl WindowsGridUi {
    pub fn new() -> WindowsGridUi {
        let (tx, rx) = channel();
        let (screen_tx, screen_rx) = channel();
        

        let (window_tx, window_rx) = channel();
        Thread::spawn(move|| {
            let instance = Instance::main_instance();
            let win = MainFrame::new(instance, "Grid UI".to_string(), tx, screen_rx).expect("Failed to create main window");
            win.show(1);
            win.update();
            
            window_tx.send(win);
            
            main_window_loop();
        });
        
        WindowsGridUi {
            window: window_rx.recv().ok().expect("Failed to create window"),
            screen_sink: screen_tx,
            input_event_source: rx,
        }
    }
}

impl GridUiInterface for WindowsGridUi {
    fn send_screen(&self, screen: Screen) {
        self.screen_sink.send(screen);
        self.window.post_message(WM_CHECK_SCREENS,0,0);
    }
    
    fn get_input_event(&self) -> InputEvent {
        self.input_event_source.recv().ok().expect("GuidUiInterface failed to receive an input event")
    }
}
