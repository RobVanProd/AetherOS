/// Input handling — keyboard from /dev/tty0 raw mode, mouse from evdev.

use std::io::Read;
use std::os::unix::io::AsRawFd;

/// Input events from keyboard and mouse.
#[derive(Debug, Clone)]
pub enum InputEvent {
    Char(char),
    Backspace,
    Enter,
    Escape,
    Tab,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Mouse { x: i32, y: i32, button: u8 },
    MouseMove { x: i32, y: i32 },
    None,
}

// Linux input event constants
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const BTN_LEFT: u16 = 0x110;

/// Raw Linux input_event (24 bytes on 64-bit).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RawInputEvent {
    tv_sec: u64,
    tv_usec: u64,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct InputReader {
    tty: std::fs::File,
    saved_termios: Option<libc::termios>,
    evdev_fds: Vec<std::fs::File>,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_buttons: u8,
    screen_width: u32,
    screen_height: u32,
}

impl InputReader {
    pub fn new() -> Result<Self, String> {
        Self::new_with_screen(1920, 1080)
    }

    pub fn new_with_screen(screen_width: u32, screen_height: u32) -> Result<Self, String> {
        // Open tty for raw keyboard input
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty0")
            .or_else(|_| {
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open("/dev/console")
            })
            .map_err(|e| format!("open tty: {e}"))?;

        let fd = tty.as_raw_fd();

        // Save and set raw mode
        let mut termios: libc::termios = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::tcgetattr(fd, &mut termios) };
        let saved = if ret == 0 { Some(termios) } else { None };

        if ret == 0 {
            let mut raw = termios;
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL | libc::BRKINT | libc::INPCK | libc::ISTRIP);
            raw.c_oflag &= !libc::OPOST;
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 0; // non-blocking
            unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) };
        }

        // Scan for evdev mouse devices
        let evdev_fds = Self::open_evdev_devices();
        if !evdev_fds.is_empty() {
            eprintln!("[input] Opened {} evdev device(s) for mouse", evdev_fds.len());
        }

        Ok(Self {
            tty,
            saved_termios: saved,
            evdev_fds,
            mouse_x: (screen_width / 2) as i32,
            mouse_y: (screen_height / 2) as i32,
            mouse_buttons: 0,
            screen_width,
            screen_height,
        })
    }

    fn open_evdev_devices() -> Vec<std::fs::File> {
        let mut fds = Vec::new();
        for i in 0..16 {
            let path = format!("/dev/input/event{}", i);
            if let Ok(file) = std::fs::OpenOptions::new()
                .read(true)
                .open(&path)
            {
                // Set non-blocking
                let fd = file.as_raw_fd();
                unsafe {
                    let flags = libc::fcntl(fd, libc::F_GETFL);
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                fds.push(file);
            }
        }
        fds
    }

    /// Non-blocking read of one input event.
    pub fn poll(&mut self) -> InputEvent {
        // Check evdev mouse first
        if let Some(ev) = self.poll_evdev() {
            return ev;
        }

        // Then check keyboard
        self.poll_keyboard()
    }

    fn poll_keyboard(&mut self) -> InputEvent {
        let mut buf = [0u8; 8];

        // Non-blocking read
        let n = match self.tty.read(&mut buf) {
            Ok(0) => return InputEvent::None,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return InputEvent::None,
            Err(_) => return InputEvent::None,
        };

        // Parse escape sequences
        if n == 1 {
            match buf[0] {
                0x1b => InputEvent::Escape,
                0x0d | 0x0a => InputEvent::Enter,
                0x7f | 0x08 => InputEvent::Backspace,
                0x09 => InputEvent::Tab,
                b if b >= 0x20 && b < 0x7f => InputEvent::Char(b as char),
                _ => InputEvent::None,
            }
        } else if n >= 3 && buf[0] == 0x1b && buf[1] == b'[' {
            match buf[2] {
                b'A' => InputEvent::Up,
                b'B' => InputEvent::Down,
                b'C' => InputEvent::Right,
                b'D' => InputEvent::Left,
                b'5' if n >= 4 && buf[3] == b'~' => InputEvent::PageUp,
                b'6' if n >= 4 && buf[3] == b'~' => InputEvent::PageDown,
                _ => InputEvent::None,
            }
        } else {
            InputEvent::None
        }
    }

    fn poll_evdev(&mut self) -> Option<InputEvent> {
        let ev_size = std::mem::size_of::<RawInputEvent>();
        let mut buf = [0u8; 24]; // size of RawInputEvent
        let mut got_mouse_move = false;
        let mut got_click: Option<InputEvent> = None;

        for file in &mut self.evdev_fds {
            loop {
                let n = match file.read(&mut buf[..ev_size]) {
                    Ok(n) if n == ev_size => n,
                    _ => break,
                };
                if n != ev_size {
                    break;
                }

                let ev: RawInputEvent = unsafe { std::ptr::read(buf.as_ptr() as *const RawInputEvent) };

                match ev.type_ {
                    EV_ABS => {
                        match ev.code {
                            ABS_X => {
                                // USB-tablet: value 0..32767 → screen X
                                self.mouse_x = (ev.value as i64 * self.screen_width as i64 / 32768) as i32;
                                self.mouse_x = self.mouse_x.clamp(0, self.screen_width as i32 - 1);
                                got_mouse_move = true;
                            }
                            ABS_Y => {
                                self.mouse_y = (ev.value as i64 * self.screen_height as i64 / 32768) as i32;
                                self.mouse_y = self.mouse_y.clamp(0, self.screen_height as i32 - 1);
                                got_mouse_move = true;
                            }
                            _ => {}
                        }
                    }
                    EV_KEY => {
                        if ev.code == BTN_LEFT {
                            if ev.value == 1 {
                                self.mouse_buttons |= 1;
                                got_click = Some(InputEvent::Mouse {
                                    x: self.mouse_x,
                                    y: self.mouse_y,
                                    button: 1,
                                });
                            } else if ev.value == 0 {
                                self.mouse_buttons &= !1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Click events take priority over move
        if let Some(click) = got_click {
            return Some(click);
        }
        if got_mouse_move {
            return Some(InputEvent::MouseMove {
                x: self.mouse_x,
                y: self.mouse_y,
            });
        }
        None
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        if let Some(termios) = self.saved_termios {
            let fd = self.tty.as_raw_fd();
            unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
        }
    }
}
