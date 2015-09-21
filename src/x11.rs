// hello_world example for x11-rs


use std::ffi::CString;
use std::mem::zeroed;
use std::ptr::{
  null,
  null_mut,
};
use std::thread;

use libc::{self, c_uint};
use x11_dl::xlib;

use input_event::InputEvent;
use screen::{Screen};
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};
use glyph_parts::glyph_to_parts;

const TITLE: &'static str = "Hello World!";
const DEFAULT_WIDTH: c_uint = 640;
const DEFAULT_HEIGHT: c_uint = 480;

#[repr(C)]
struct pollfd {
    fd: libc::c_int,
    events: libc::c_short,
    revents: libc::c_short,
}

extern "C" {
    fn pipe2(pipefd: *const libc::c_int, flags: libc::c_int);

    fn read(fd: libc::c_int, buf: *mut libc::c_void, count: libc::c_uint) -> libc::c_int;
    fn write(fd: libc::c_int, buf: *const libc::c_void, count: libc::c_uint) -> libc::c_int;

    fn poll(fds: *mut pollfd, nfds: libc::c_uint, timeout: libc::c_int) -> libc::c_int;

}




pub struct GridUi {
    screen_sink: Sender<Screen>,
    pub input_event_source: Receiver<InputEvent>,

    /// Pipe file descriptor. Writing to this signals the UI thread to check screen_source
    write_pipe: libc::c_int,
}

impl GridUi {
    pub fn new() -> GridUi {
        let (read_pipe, write_pipe) = {
            let mut pipes: [libc::c_int;2] = [0,0];
            unsafe { pipe2(pipes.as_mut_ptr(), 2048 /* O_NONBLOCK*/ ); }

            (pipes[0], pipes[1])
        };

        let (screen_sink, screen_source) = channel();
        let (input_event_sink, input_event_source) = channel();

        thread::spawn(move || {
            unsafe { ui_main(read_pipe, screen_source, input_event_sink); }
        });

        GridUi{
            screen_sink: screen_sink,
            input_event_source: input_event_source,
            write_pipe: write_pipe,
        }
    }

    pub fn send_screen(&mut self, screen: Screen) {
        self.screen_sink.send(screen).ok().expect("Could not send screen to window, which has unexpectedly closed");

        let buf = [0u8];
        unsafe {
            write(self.write_pipe, buf.as_ptr() as *const libc::c_void, 1); 
        }
    }
}

unsafe fn ui_main(signal_fd: libc::c_int, screen_source: Receiver<Screen>, input_event_sink: Sender<InputEvent>) {
    // Open Xlib library
    let xlib = xlib::Xlib::open().unwrap();

    // Open display
    let display = (xlib.XOpenDisplay)(null());
    if display == null_mut() {
      panic!("can't open display");
    }

    // Load atoms
    let wm_delete_window_str = CString::new("WM_DELETE_WINDOW").unwrap();
    let wm_protocols_str = CString::new("WM_PROTOCOLS").unwrap();

    let wm_delete_window = (xlib.XInternAtom)(display, wm_delete_window_str.as_ptr(), xlib::False);
    let wm_protocols = (xlib.XInternAtom)(display, wm_protocols_str.as_ptr(), xlib::False);

    if wm_delete_window == 0 || wm_protocols == 0 {
      panic!("can't load atoms");
    }

    // Create window
    let screen_num = (xlib.XDefaultScreen)(display);
    let root = (xlib.XRootWindow)(display, screen_num);
    let white_pixel = (xlib.XWhitePixel)(display, screen_num);

    let mut attributes: xlib::XSetWindowAttributes = zeroed();
    attributes.background_pixel = white_pixel;

    let window = (xlib.XCreateWindow)(display, root, 0, 0, DEFAULT_WIDTH, DEFAULT_HEIGHT, 0, 0,
                                      xlib::InputOutput as c_uint, null_mut(),
                                      xlib::CWBackPixel, &mut attributes);
    (xlib.XSelectInput)(display, window, xlib::ExposureMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::KeyPressMask | xlib::KeyReleaseMask);

    // Set window title
    let title_str = CString::new(TITLE).unwrap();
    (xlib.XStoreName)(display, window, title_str.as_ptr() as *mut _);

    // Subscribe to delete (close) events
    let mut protocols = [wm_delete_window];

    if (xlib.XSetWMProtocols)(display, window, &mut protocols[0] as *mut xlib::Atom, 1)
       == xlib::False
    {
      panic!("can't set WM protocols");
    }

    // Show window
    (xlib.XMapWindow)(display, window);

    // Main loop
    let mut event: xlib::XEvent = zeroed();


    let connection_number = (xlib.XConnectionNumber)(display);
    println!("Connection_number = {}", connection_number);

    let mut screen = Screen { glyphs: Vec::new(), width: 0 }; 

    let xs = include_bytes!("glyphs.bin");//[0x81u8, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x81];
    let bit_count = (xs.len() as u32) * 8;
    let bits_per_glyph = 20*40;
    let glyph_count = bit_count / bits_per_glyph;
    let glyph_bitmap = (xlib.XCreateBitmapFromData)(display, window, xs.as_ptr() as *const i8, 20,40*glyph_count);

    'event_loop: loop {
      if (xlib.XPending)(display) == 0 {
          let mut poll_fds: [pollfd;2] = [
              pollfd{
                  fd: signal_fd,
                  events: 1,
                  revents: 0,
              },
              pollfd{
                  fd: connection_number,
                  events: 1,
                  revents: 0,
              },
          ];
          poll(poll_fds.as_mut_ptr(), 2, -1);
          let mut need_expose = false;
          if poll_fds[0].revents != 0 {
              let mut buf = [0u8;10];
              loop {
                  if read(signal_fd, buf.as_mut_ptr() as *mut libc::c_void, 10) <= 0 {
                      break;
                  }
              }

              loop {
                  match screen_source.try_recv() {
                      Err(TryRecvError::Empty) => { break; },
                      Err(TryRecvError::Disconnected) => { break 'event_loop; }
                      Ok(new_screen) => { 
                          screen = new_screen;
                          println!("Screen = {:?}", screen.glyphs);
                          need_expose = true;
                      }
                  }
              }
          }

          if need_expose {
              let evt: xlib::XEvent = zeroed();
              let mut expose_evt : xlib::XExposeEvent = evt.into();
              expose_evt.type_ = xlib::Expose; 
              expose_evt.width = 100;
              expose_evt.width = 100;
              expose_evt.display = display;
              expose_evt.window = window;

              (xlib.XSendEvent)(display, window, xlib::False, xlib::ExposureMask, &mut expose_evt.into());
          }

          continue;
      }

      (xlib.XNextEvent)(display, &mut event);
      match event.get_type() {
        xlib::ClientMessage => {
          let xclient: xlib::XClientMessageEvent = From::from(event);

          // WM_PROTOCOLS client message
          if xclient.message_type == wm_protocols && xclient.format == 32 {
            let protocol = xclient.data.get_long(0) as xlib::Atom;

            // WM_DELETE_WINDOW (close event)
            if protocol == wm_delete_window {
              break;
            }
          }
        },

        xlib::KeyPress => {
            let pressevent: xlib::XKeyEvent = event.into();
            println!("Key press! {}", pressevent.keycode);
        }

        xlib::Expose => {
          let gc = (xlib.XDefaultGC)(display, screen_num);
/*          let p = (xlib.XCreatePixmap)(display, window, 100, 100, 24);
          println!("p = {:?}, gc = {:?}", p, gc);
          //(xlib.XPutPixel)(p, 0,0,0xff00ff);
          let img = (xlib.XGetImage)(display, p, 0, 0, 10, 10, 0xffffffff, xlib::ZPixmap);
          println!("img = {:?}  {}x{}", img, (*img).width, (*img).height);
          println!("bytes_per_line = {}, depth = {}", (*img).bytes_per_line, (*img).depth);
          println!("pixel initially = {:?}", (xlib.XGetPixel)(img, 0,0));
          println!("masks = {:x}, {:x}, {:x}", (*img).red_mask, (*img).green_mask, (*img).blue_mask);
          for x in (0..10) {
              for y in (0..10) {
                  (xlib.XPutPixel)(img, x,y, 0xd0ff0f);
              }
          }
          println!("pixel finally = {:x}", (xlib.XGetPixel)(img, 0,0));


          (xlib.XDrawLine)(display, p, gc, 0,0,40,40);

          (xlib.XPutImage)(display, p, gc, img, 0,0, 20,20, 0,0);
          (xlib.XCopyArea)(display, p, window, gc, 0,0, 100,100, 0,20);
          (xlib.XCopyArea)(display, p, window, gc, 0,0, 100,100, 100,20);
          (xlib.XFreePixmap)(display, p);
*/
          let mut x = 0u32;
          let mut y = 0u32;
          for glyph in screen.glyphs.iter() {
              (xlib.XSetForeground)(display, gc, glyph.foreground as u64);
              (xlib.XSetBackground)(display, gc, glyph.background as u64);

              let parts = glyph_to_parts(glyph.character); 
              println!("Drawing parts {:?}", parts);
              (xlib.XCopyPlane)(display, glyph_bitmap, window, gc, 0,40*(parts[0] as i32), 20,40, (x as i32)*20,(y as i32)*40, 1);

              x += 1;
              if x==screen.width {
                  x = 0;
                  y += 1;
              }
          }
//          (xlib.XDestroyImage)(img); 
        }
        _ => {},
      }
    }

    // Clean up
    (xlib.XFreePixmap)(display, glyph_bitmap);
    (xlib.XDestroyWindow)(display, window);
    (xlib.XCloseDisplay)(display);
}

#[test]
fn x11test() {
    let mut gridui = GridUi::new();


    let mut i = 1;
    loop {
        i = i + 1;
        if i==10 { i=1; }
        let screen = Screen{
            glyphs: vec![ 
                Glyph{ character: i, foreground: 0xff, background: 0xff00 },
                Glyph{ character: i+1, foreground: 0, background: 0xffffff },

            ],
            width: 2,
        };
        gridui.send_screen(screen);

        thread::sleep_ms(1000);
    }
}

