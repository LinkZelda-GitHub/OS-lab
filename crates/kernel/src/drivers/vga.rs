//! VGA Framebuffer Driver
//!
//! Provides low-level access to the linear framebuffer obtained from UEFI GOP.
//! Supports pixel drawing, rectangle fill, screen clear, scroll operations,
//! and integration with the `embedded-graphics` crate.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use boot::GopInfo;
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::{PixelColor, raw::RawU32},
    prelude::OriginDimensions,
};

use crate::memory::physical_to_virtual;

/// Represents a 32-bit BGRx color (matching the UEFI GOP framebuffer format)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    pub const BLACK: Self = Self(0x00000000);
    pub const WHITE: Self = Self(0x00FFFFFF);
    pub const RED: Self = Self(0x00FF0000);
    pub const GREEN: Self = Self(0x0000FF00);
    pub const BLUE: Self = Self(0x000000FF);
    pub const YELLOW: Self = Self(0x00FFFF00);
    pub const CYAN: Self = Self(0x0000FFFF);
    pub const MAGENTA: Self = Self(0x00FF00FF);
    pub const GRAY: Self = Self(0x00808080);
    pub const DARK_GRAY: Self = Self(0x00404040);
    pub const LIGHT_GRAY: Self = Self(0x00C0C0C0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(0x00000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b)
    }

    /// Get the raw u32 value (BGRx format)
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

// --- embedded-graphics PixelColor integration ---

impl PixelColor for Color {
    type Raw = RawU32;
}

/// Wrapper around `VgaDriver` that implements `embedded_graphics::DrawTarget`.
///
/// Acquire this via `VgaDriver::display()` while holding the VGA lock.
pub struct VgaDisplay<'a> {
    vga: &'a VgaDriver,
}

impl<'a> VgaDisplay<'a> {
    pub fn new(vga: &'a VgaDriver) -> Self {
        Self { vga }
    }
}

impl OriginDimensions for VgaDisplay<'_> {
    fn size(&self) -> Size {
        Size::new(self.vga.width, self.vga.height)
    }
}

impl DrawTarget for VgaDisplay<'_> {
    type Color = Color;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels {
            if x >= 0 && y >= 0 {
                self.vga.draw_pixel(x as u32, y as u32, color);
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let x0 = area.top_left.x.max(0) as u32;
        let y0 = area.top_left.y.max(0) as u32;
        let w = (area.size.width as u32).min(self.vga.width - x0);
        let h = (area.size.height as u32).min(self.vga.height - y0);

        let mut colors = colors.into_iter();
        for py in y0..(y0 + h) {
            unsafe {
                let row_base = (py as usize) * (self.vga.stride_bytes as usize);
                for px in x0..(x0 + w) {
                    if let Some(color) = colors.next() {
                        let c = color.0;
                        let base = row_base + (px as usize) * 4;
                        self.vga.fb.add(base).write_volatile((c & 0xFF) as u8); // B
                        self.vga
                            .fb
                            .add(base + 1)
                            .write_volatile(((c >> 8) & 0xFF) as u8); // G
                        self.vga
                            .fb
                            .add(base + 2)
                            .write_volatile(((c >> 16) & 0xFF) as u8); // R
                    }
                }
            }
        }
        Ok(())
    }
}

/// The VGA framebuffer driver
pub struct VgaDriver {
    /// Virtual address of the framebuffer (byte-level access)
    fb: *mut u8,
    /// Framebuffer size in bytes
    fb_size: u64,
    /// Horizontal resolution
    width: u32,
    /// Vertical resolution
    height: u32,
    /// Bytes per scanline
    stride_bytes: u32,
    /// Cursor X position (in character cells)
    cursor_x: AtomicU32,
    /// Cursor Y position (in character cells)
    cursor_y: AtomicU32,
}

// SAFETY: The framebuffer is memory-mapped I/O accessible from any CPU core
unsafe impl Send for VgaDriver {}
unsafe impl Sync for VgaDriver {}

impl VgaDriver {
    /// Create a new VGA driver from GOP info
    ///
    /// # Safety
    /// Caller must ensure gop_info contains valid framebuffer address and
    /// parameters.
    pub unsafe fn new(gop_info: &GopInfo) -> Self {
        let fb_virt = physical_to_virtual(gop_info.fb_addr) as *mut u8;
        // GOP stride is in pixels; framebuffer is 4 bytes per pixel (BGRx)
        let stride_bytes = gop_info.stride * 4;

        info!(
            "VGA: fb_virt=0x{:x}, w={}, h={}, stride_bytes={}, fb_size={}",
            fb_virt as u64, gop_info.width, gop_info.height, stride_bytes, gop_info.fb_size
        );

        Self {
            fb: fb_virt,
            fb_size: gop_info.fb_size,
            width: gop_info.width,
            height: gop_info.height,
            stride_bytes,
            cursor_x: AtomicU32::new(0),
            cursor_y: AtomicU32::new(0),
        }
    }

    /// Create an `embedded-graphics` compatible display wrapper
    #[inline]
    pub fn display(&self) -> VgaDisplay<'_> {
        VgaDisplay::new(self)
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn stride_pixels(&self) -> u32 {
        self.stride_bytes / 4
    }

    /// Raw framebuffer pointer (for direct memory operations)
    #[inline]
    pub fn fb_ptr(&self) -> *mut u32 {
        self.fb as *mut u32
    }

    /// Get cursor position in pixels
    pub fn cursor_pos(&self) -> (u32, u32) {
        (
            self.cursor_x.load(Ordering::Relaxed),
            self.cursor_y.load(Ordering::Relaxed),
        )
    }

    /// Set cursor position in pixels
    pub fn set_cursor(&self, x: u32, y: u32) {
        self.cursor_x
            .store(x.min(self.width - 1), Ordering::Relaxed);
        self.cursor_y
            .store(y.min(self.height - 1), Ordering::Relaxed);
    }

    /// Draw a single pixel at (x, y) with given color.
    /// Writes individual bytes for BGRx framebuffer format.
    #[inline]
    pub fn draw_pixel(&self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            unsafe {
                let base = (y as usize) * (self.stride_bytes as usize) + (x as usize) * 4;
                let c = color.0;
                self.fb.add(base).write_volatile((c & 0xFF) as u8); // B
                self.fb
                    .add(base + 1)
                    .write_volatile(((c >> 8) & 0xFF) as u8); // G
                self.fb
                    .add(base + 2)
                    .write_volatile(((c >> 16) & 0xFF) as u8); // R
            }
        }
    }

    /// Fill a rectangle with given color
    pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: Color) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        let c = color.0;
        let b = (c & 0xFF) as u8;
        let g = ((c >> 8) & 0xFF) as u8;
        let r = ((c >> 16) & 0xFF) as u8;

        for py in y..y_end {
            unsafe {
                let row_base = (py as usize) * (self.stride_bytes as usize);
                for px in x..x_end {
                    let base = row_base + (px as usize) * 4;
                    self.fb.add(base).write_volatile(b);
                    self.fb.add(base + 1).write_volatile(g);
                    self.fb.add(base + 2).write_volatile(r);
                }
            }
        }
    }

    /// Clear the entire screen with given color
    pub fn clear_screen(&self, color: Color) {
        let c = color.0;
        let b = (c & 0xFF) as u8;
        let g = ((c >> 8) & 0xFF) as u8;
        let r = ((c >> 16) & 0xFF) as u8;
        for y in 0..self.height {
            unsafe {
                let row_base = (y as usize) * (self.stride_bytes as usize);
                for x in 0..self.width {
                    let base = row_base + (x as usize) * 4;
                    self.fb.add(base).write_volatile(b);
                    self.fb.add(base + 1).write_volatile(g);
                    self.fb.add(base + 2).write_volatile(r);
                }
            }
        }
        self.cursor_x.store(0, Ordering::Relaxed);
        self.cursor_y.store(0, Ordering::Relaxed);
    }

    /// Scroll the screen up by `lines` lines of `char_height` pixels each
    pub fn scroll_up(&self, lines: u32, char_height: u32, bg_color: Color) {
        let pixel_lines = lines * char_height;
        if pixel_lines >= self.height {
            self.clear_screen(bg_color);
            return;
        }
        let row_bytes = (self.width as usize) * 4;

        for y in 0..(self.height - pixel_lines) {
            let src_y = y + pixel_lines;
            unsafe {
                let src = self.fb.add((src_y as usize) * (self.stride_bytes as usize));
                let dst = self.fb.add((y as usize) * (self.stride_bytes as usize));
                core::ptr::copy_nonoverlapping(src, dst, row_bytes);
            }
        }
        let clear_start_y = self.height - pixel_lines;
        self.fill_rect(0, clear_start_y, self.width, pixel_lines, bg_color);
    }

    /// Copy a rectangular region (unused, kept for completeness)
    pub fn copy_rect(&self, src_x: u32, src_y: u32, dst_x: u32, dst_y: u32, w: u32, h: u32) {
        let w = w.min(self.width - src_x).min(self.width - dst_x);
        let h = h.min(self.height - src_y).min(self.height - dst_y);
        let sb = self.stride_bytes;

        let y_range: Vec<u32> = if src_y > dst_y {
            (0..h).collect()
        } else {
            (0..h).rev().collect()
        };

        for &dy in &y_range {
            for dx in 0..w {
                let sx = src_x + dx;
                let sy = src_y + dy;
                let dx = dst_x + dx;
                let dy = dst_y + dy;
                if sx < self.width && sy < self.height && dx < self.width && dy < self.height {
                    unsafe {
                        let src_off = (sy as usize) * (sb as usize) + (sx as usize) * 4;
                        let b = self.fb.add(src_off).read_volatile();
                        let g = self.fb.add(src_off + 1).read_volatile();
                        let r = self.fb.add(src_off + 2).read_volatile();
                        let dst_off = (dy as usize) * (sb as usize) + (dx as usize) * 4;
                        self.fb.add(dst_off).write_volatile(b);
                        self.fb.add(dst_off + 1).write_volatile(g);
                        self.fb.add(dst_off + 2).write_volatile(r);
                    }
                }
            }
        }
    }
}

// Global VGA driver instance
once_mutex!(pub VGA: VgaDriver);
guard_access_fn!(pub get_vga(VGA: VgaDriver));

/// Initialize the VGA driver
pub fn init(gop_info: &GopInfo) {
    let driver = unsafe { VgaDriver::new(gop_info) };
    init_VGA(driver);
    get_vga_for_sure().clear_screen(Color::BLACK);
    info!(
        "VGA Driver Initialized: {}x{}",
        gop_info.width, gop_info.height
    );
}
