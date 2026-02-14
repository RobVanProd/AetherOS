/// Linux framebuffer interface — open /dev/fb0, ioctl for screen info, mmap, double-buffer, blit.

use std::fs::{File, OpenOptions};
use std::num::NonZeroUsize;
use std::os::unix::io::AsRawFd;
use std::ptr::NonNull;

use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};

use crate::theme;

/// Framebuffer screen info (from FBIOGET_VSCREENINFO / FBIOGET_FSCREENINFO).
#[derive(Debug, Clone)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub stride: u32, // bytes per line
    pub bpp: u32,    // bits per pixel
}

// Linux framebuffer ioctls
const FBIOGET_VSCREENINFO: libc::c_int = 0x4600;
const FBIOGET_FSCREENINFO: libc::c_int = 0x4602;

#[repr(C)]
#[derive(Default)]
struct FbVarScreenInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,
    bits_per_pixel: u32,
    grayscale: u32,
    red: FbBitfield,
    green: FbBitfield,
    blue: FbBitfield,
    transp: FbBitfield,
    nonstd: u32,
    activate: u32,
    height: u32,
    width: u32,
    accel_flags: u32,
    // timing fields
    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[repr(C)]
#[derive(Default)]
struct FbBitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
#[derive(Default)]
struct FbFixScreenInfo {
    id: [u8; 16],
    smem_start: libc::c_ulong,
    smem_len: u32,
    fb_type: u32,
    type_aux: u32,
    visual: u32,
    xpanstep: u16,
    ypanstep: u16,
    ywrapstep: u16,
    line_length: u32,
    mmio_start: libc::c_ulong,
    mmio_len: u32,
    accel: u32,
    capabilities: u16,
    reserved: [u16; 2],
}

pub struct Framebuffer {
    _file: File,
    fb_ptr: *mut u8,
    fb_len: usize,
    pub info: ScreenInfo,
    back_buffer: Vec<u8>,
    prev_buffer: Vec<u8>,
    dirty: bool,
}

unsafe impl Send for Framebuffer {}

impl Framebuffer {
    pub fn open(path: &str) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("open {path}: {e}"))?;

        let fd = file.as_raw_fd();

        // Get variable screen info
        let mut vinfo = FbVarScreenInfo::default();
        let ret = unsafe { libc::ioctl(fd, FBIOGET_VSCREENINFO, &mut vinfo as *mut _) };
        if ret < 0 {
            return Err(format!("FBIOGET_VSCREENINFO failed: {}", std::io::Error::last_os_error()));
        }

        // Get fixed screen info
        let mut finfo = FbFixScreenInfo::default();
        let ret = unsafe { libc::ioctl(fd, FBIOGET_FSCREENINFO, &mut finfo as *mut _) };
        if ret < 0 {
            return Err(format!("FBIOGET_FSCREENINFO failed: {}", std::io::Error::last_os_error()));
        }

        let info = ScreenInfo {
            width: vinfo.xres,
            height: vinfo.yres,
            stride: finfo.line_length,
            bpp: vinfo.bits_per_pixel,
        };

        let fb_len = (finfo.line_length * vinfo.yres) as usize;

        // mmap the framebuffer
        let fb_nonnull = unsafe {
            mmap(
                None,
                NonZeroUsize::new(fb_len).ok_or("zero fb size")?,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                &file,
                0,
            )
            .map_err(|e| format!("mmap: {e}"))?
        };
        let fb_ptr = fb_nonnull.as_ptr() as *mut u8;

        let back_buffer = vec![0u8; fb_len];
        let prev_buffer = vec![0xFFu8; fb_len]; // init different so first frame is dirty

        eprintln!(
            "[fb] Opened {path}: {}x{} bpp={} stride={}",
            info.width, info.height, info.bpp, info.stride
        );

        Ok(Self {
            _file: file,
            fb_ptr,
            fb_len,
            info,
            back_buffer,
            prev_buffer,
            dirty: true,
        })
    }

    /// Get a mutable slice to the back buffer for rendering.
    pub fn back_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.back_buffer
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.info.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// Mark the back buffer as dirty (call after drawing).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Returns true if the back buffer differs from the previous frame.
    pub fn is_dirty(&self) -> bool {
        self.dirty || self.back_buffer != self.prev_buffer
    }

    /// Blit the back buffer to the framebuffer (RGBA → BGRA conversion).
    /// Skips the blit entirely if nothing changed.
    pub fn present(&mut self) {
        if !self.is_dirty() {
            return;
        }

        // tiny-skia renders RGBA premultiplied. Linux fb is typically BGRA (or BGRX).
        // Swap R and B channels using 4-byte chunks for speed.
        let dst = unsafe { std::slice::from_raw_parts_mut(self.fb_ptr, self.fb_len) };

        // Fast path: 32bpp, process 4 bytes at a time
        for (src_px, dst_px) in self.back_buffer.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
            dst_px[0] = src_px[2]; // B
            dst_px[1] = src_px[1]; // G
            dst_px[2] = src_px[0]; // R
            dst_px[3] = src_px[3]; // A
        }

        self.prev_buffer.copy_from_slice(&self.back_buffer);
        self.dirty = false;
    }

    /// Fill entire back buffer with a solid color.
    pub fn clear(&mut self, color: theme::Color) {
        let stride = self.info.stride as usize;
        let w = self.info.width as usize;
        let bpp = (self.info.bpp / 8) as usize;
        for y in 0..self.info.height as usize {
            for x in 0..w {
                let off = y * stride + x * bpp;
                self.back_buffer[off] = color.r;
                self.back_buffer[off + 1] = color.g;
                self.back_buffer[off + 2] = color.b;
                if bpp >= 4 {
                    self.back_buffer[off + 3] = color.a;
                }
            }
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if self.fb_len > 0 {
            if let Some(nn) = NonNull::new(self.fb_ptr as *mut libc::c_void) {
                unsafe {
                    let _ = munmap(nn, self.fb_len);
                }
            }
        }
    }
}
