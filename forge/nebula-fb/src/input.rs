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

pub struct InputReader {
    tty: std::fs::File,
    saved_termios: Option<libc::termios>,
}

impl InputReader {
    pub fn new() -> Result<Self, String> {
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

        Ok(Self {
            tty,
            saved_termios: saved,
        })
    }

    /// Non-blocking read of one input event.
    pub fn poll(&mut self) -> InputEvent {
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
}

impl Drop for InputReader {
    fn drop(&mut self) {
        if let Some(termios) = self.saved_termios {
            let fd = self.tty.as_raw_fd();
            unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
        }
    }
}
