#![feature(globs, macro_rules, phase)]

#[phase(plugin, link)]
extern crate log;

extern crate libc;

#[phase(plugin, link)]
extern crate "rust-windows" as windows;

#[phase(plugin, link)]
extern crate "mips_cpu" as mips;

use std::ptr;
use std::cell::{RefCell};
use std::comm::channel;
use std::io::File;

use libc::{c_int};

use windows::main_window_loop;
use windows::ll::types::{UINT, HBRUSH, COLORREF, LPARAM, WPARAM, LRESULT};
use windows::ll::all::{PostQuitMessage, GetSysColor, CREATESTRUCT};
use windows::ll::gdi::{GetStockObject, SetDCBrushColor};
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

use mips::cpu::{MipsCpu, FaultType};

// TODO duplicate of hello.rc
static IDI_ICON: int = 0x101;
static MENU_MAIN: int = 0x201;
//static MENU_NEW: int = 0x202;
//static MENU_EXIT: int = 0x203;

#[deriving(Show)]
enum InputEvent {
    Close,
    MouseDown(u32, u32),
    MouseUp(u32, u32),
    KeyDown(u32),
    Size(u32, u32),
}

#[deriving(Copy, Clone, Show)]
struct Glyph {
    character: u32,
    background: u32,
    foreground: u32,
}

#[deriving(Clone)]
struct Screen {
    glyphs: Vec<Glyph>,
    width: uint,
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
    grid_height: uint,
    state: RefCell<MainFrameState>,
}

const WM_CHECK_SCREENS : UINT = 0x0401;

wnd_proc!(MainFrame, win, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_KEYDOWN, WM_KEYUP, WM_SIZE, WM_PAINT, WM_ERASEBKGND, ANY)

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

fn character_to_unicode(character: u32) -> char {
    let lower_a_code = 0x1000;
    let after_lowers = lower_a_code + 26*16;
    
    if lower_a_code <= character && character  < after_lowers && (character & 0x0f)==0 {
        return ((('a' as u32) + ((character & 0x0ff0)>>4)-1) as u8) as char;
    }
    
    let case_mask = 0x00002000;
    let upper_a_code = lower_a_code ^ case_mask;
    let after_uppers = after_lowers ^ case_mask;
    if upper_a_code <= character && character < after_uppers && (character & 0x0f)==0 {
        return ((('A' as u32) + ((character & 0x0ff0)>>4)-1) as u8) as char;
    }
    
    match character {
        0 => ' ',
        1 => '_',
        2 => '-',
        3 => '.',
        4 => ',',
        5 => '/',
        6 => '\\',
        7 => ':',
        8 => ';',
        9 => '~',
        
        10 => '0',
        11 => '1',
        12 => '2',
        13 => '3',
        14 => '4',
        15 => '5',
        16 => '6',
        17 => '7',
        18 => '8',
        19 => '9',
        _ => '?',
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
                for (row_idx, row) in screen.glyphs.chunks(screen.width).enumerate() {
                    for (col_idx, cell) in row.iter().enumerate() {
                        pdc.dc.set_text_color(cell.foreground as COLORREF);
                        pdc.dc.set_background_color(cell.background as COLORREF);
                        
                        pdc.dc.text_out((col_idx*grid_width) as int, (row_idx*self.grid_height) as int, String::from_char(1, character_to_unicode(cell.character)).as_slice());
                    }
                }
            }
            
            if let Some(client_rect) = self.win.client_rect(){
                let max_filled_x = screen.width * grid_width;
                let max_filled_y = if screen.width>0 { (screen.glyphs.len() / screen.width) * self.grid_height } else { 0 };
                
                let filler_color = unsafe { GetSysColor(15 /* COLOR_3DFACE */) as COLORREF };
                unsafe { SetDCBrushColor(pdc.dc.raw, filler_color) };
                let null_pen = unsafe { GetStockObject(8 /* NULL_PEN */) };
                let dc_brush = unsafe { GetStockObject(18 /* DC_BRUSH */) };
                
                pdc.dc.select_object(null_pen);
                pdc.dc.select_object(dc_brush);
                pdc.dc.rect((max_filled_x as int, 0), (client_rect.right as int + 1, client_rect.bottom as int + 1));
                pdc.dc.rect((0, max_filled_y as int), (max_filled_x as int + 1, client_rect.bottom as int + 1));
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
    fn on_left_button_down(&self, x: int, y: int, _flags: u32) {
        let grid_width = self.grid_height/2;
        let col = (x as u32) / (grid_width as u32);
        let row = (y as u32) / (self.grid_height as u32);
        self.input_sink.send(InputEvent::MouseDown(col as u32, row as u32));
    }
}

impl OnLeftButtonUp for MainFrame {
    fn on_left_button_up(&self, x: int, y: int, _flags: u32) {
        let grid_width = self.grid_height/2;
        let col = (x as u32) / (grid_width as u32);
        let row = (y as u32) / (self.grid_height as u32);
        self.input_sink.send(InputEvent::MouseUp(col as u32, row as u32));
    }
}

impl OnKeyDown for MainFrame {
    fn on_key_down(&self, keycode: u8, flags: u32) -> bool {
        println!("Key down {} {}", keycode, flags);
        return true;
    }
}

impl OnKeyUp for MainFrame {
    fn on_key_up(&self, keycode: u8, flags: u32) -> bool {
        println!("Key up {} {}", keycode, flags);
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
        let icon = Image::load_resource(instance, IDI_ICON, ImageType::IMAGE_ICON, 0, 0);
        let wnd_class = WndClass {
            classname: "MainFrame".to_string(),
            style: 0x0001 | 0x0002, // CS_HREDRAW | CS_VREDRAW
            icon: icon,
            icon_small: None,
            cursor: Image::load_cursor_resource(32512), // standard arrow
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
            screen_source: screen_source,
            state: RefCell::new(MainFrameState{
                screen: Screen{
                    width:0,
                    glyphs: Vec::new()
                },
                announced_grid_size: (-1,-1),  
            }),
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
    
    fn with_state_mut(&self, f: |&mut MainFrameState|) {
        if let Some(mut state) = self.state.try_borrow_mut() {
            f(state.deref_mut());
        }
    }
    
    fn with_state(&self, f: |&MainFrameState|) {
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

trait GridUiInterface {
    fn send_screen(&self, screen: Screen);
    fn get_input_event(&self) -> InputEvent;
}

struct WindowsGridUi {
    screen_sink: Sender<Screen>,
    input_event_source: Receiver<InputEvent>,
    window: Window,
}

impl WindowsGridUi {
    fn new() -> WindowsGridUi {
        let (tx, rx) = channel();
        let (screen_tx, screen_rx) = channel();
        
        let instance = Instance::main_instance();

        let (window_tx, window_rx) = channel();
        spawn(move|| {
            let win = MainFrame::new(instance, "Grid UI".to_string(), tx, screen_rx).expect("Failed to create main window");
            win.show(1);
            win.update();
            
            window_tx.send(win);
            
            let exit_code = main_window_loop();
            std::os::set_exit_status(exit_code as int);
        });
        
        WindowsGridUi {
            window: window_rx.recv(),
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
        self.input_event_source.recv()
    }
}

fn place_code(cpu: &mut MipsCpu, code: &[u8], index: u32) {
    // Assumed code length is a multiple of 4
    for (offset, chunk) in code.chunks(4).enumerate() {
        let val = (chunk[0] as u32) | (chunk[1] as u32 << 8) | (chunk[2] as u32 << 16) | (chunk[3] as u32 << 24);
        cpu.set_mem(index + 4*(offset as u32), val);
    }
}

// Things that the application can do wrong to get it crashed
#[deriving(Show)]
enum ApplicationException {
    InvalidSyscall,
    ScreenTooBig,
    UnalignedScreenMemory,
    ClosedByUi,
}

struct UiRunner {
    ui: WindowsGridUi, // TODO: be generic over different UIs
    cpu: MipsCpu,
    screen_width: u32,
    screen_height: u32,
    cursor_down: bool,
    cursor_x: u32,
    cursor_y: u32,
}

const MAXIMUM_SCREEN_MEMORY : u32 = 400*400;
impl UiRunner {
    fn new(code: Vec<u8>) -> UiRunner {
        let ui = WindowsGridUi::new();
        
        let mut cpu = MipsCpu::new();
        
        place_code(&mut cpu, code.as_slice(), 0);
        
        UiRunner {
            ui: ui,
            cpu: cpu,
            screen_width: 0,
            screen_height: 0,
            cursor_down: false,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
    
    fn run(&mut self) {
        loop {
            if let Some(interrupt) = self.cpu.run(40) {
                println!("Interrupt: {}", interrupt);
                match interrupt {
                    FaultType::Syscall => {
                        if let Some(error) = self.dispatch_syscall() {
                            println!("CPU halting executing with {}", error);
                            break;
                        }
                        self.cpu.clear_fault();
                    },
                    _ => {
                        
                    }
                }
            } else {
                // Signal that the CPU is busy
                println!("CPU hung");
                break;
            }
        }
    }
    
    fn dispatch_syscall(&mut self) -> Option<ApplicationException> {
        println!("Issued syscall: {}", self.cpu.regs[2]);
        let syscall_number = self.cpu.regs[2];
        let syscall_arg = self.cpu.regs[3];
        match syscall_number {
            1 => { // Show screen
                let address_base = syscall_arg;
                if address_base%4 != 0 {
                    return Some(ApplicationException::UnalignedScreenMemory);
                }
                let width = self.cpu.read_mem(address_base);
                let height = self.cpu.read_mem(address_base+4);
                let size = (width as u64)*(height as u64);
                if size >= MAXIMUM_SCREEN_MEMORY as u64 {
                    return Some(ApplicationException::ScreenTooBig);
                }
                let glyph_address_base = address_base + 8;
                let glyphs = Vec::from_fn(size as uint, |idx| {
                    let glyph_start_address = glyph_address_base + (idx as u32)*12;
                    Glyph {
                        character: self.cpu.read_mem(glyph_start_address),
                        foreground: self.cpu.read_mem(glyph_start_address + 4),
                        background: self.cpu.read_mem(glyph_start_address + 8),
                    }
                });
                println!("got {}", glyphs);
                self.ui.send_screen(Screen{
                    width: width as uint,
                    glyphs: glyphs
                });
                None
            }
            2 => { // Get input event
                match self.get_application_input_event() {
                    Some(input_event_type) => { self.cpu.regs[2] = input_event_type; }
                    None => { return Some(ApplicationException::ClosedByUi); }
                }
                None
            }
            _ => {
                // Maybe crash the application here?
                println!("Unrecognized syscall: {}", syscall_number);
                Some(ApplicationException::InvalidSyscall)
            }
        }
        
    }
    
    fn get_application_input_event(&mut self) -> Option<u32> {
        loop {
            match self.ui.get_input_event() {
                InputEvent::Close => { return None; }
                InputEvent::Size(cols, rows) => {
                    println!("Resized to {} by {}", cols, rows);
                    self.screen_width = cols;
                    self.screen_height = rows;
                    return Some(1);
                }
                InputEvent::MouseDown(col, row) => {
                    self.cursor_down = true;
                    self.cursor_x = col;
                    self.cursor_y = row;
                    return Some(2);
                }
                InputEvent::MouseUp(col, row) => {
                    self.cursor_down = false;
                    self.cursor_x = col;
                    self.cursor_y = row;
                    return Some(3);
                }
                x => {
                    println!("{}", x);
                }
            }
        }
    }
}

fn run_window(code: Vec<u8>) {
    UiRunner::new(code).run();
}

fn main() {
    let contents = File::open(&Path::new("asm/mips1-repositioned.bin")).read_to_end().unwrap_or_else(|_| { panic!("Could not read MIPS code"); } );
    run_window(contents);
}
