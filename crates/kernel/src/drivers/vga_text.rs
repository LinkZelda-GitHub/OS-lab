//! VGA Text Console
//!
//! Renders text onto the VGA framebuffer using a built-in 8x16 bitmap font.
//! Supports cursor management, line wrapping, scrolling, and color themes.
//!
//! Holds its own framebuffer parameters so text output is never dropped even
//! when another process holds the VGA mutex.
//!
//! Supports a fixed header area (for YSOS banner) and a scrollable body with
//! a scrollback buffer so users can page through history with arrow keys.

use alloc::collections::VecDeque;
use alloc::string::String;
use core::fmt;

use crate::drivers::{
    font8x16::FONT_8X16,
    vga::{Color, get_vga_for_sure},
};

/// Maximum number of lines kept in the scrollback buffer
const MAX_SCROLLBACK: usize = 1000;

/// Number of fixed header rows (YSOS banner area, not scrollable)
const HEADER_ROWS: u32 = 8;

/// Text console rendering on the VGA framebuffer.
/// Stores its own framebuffer reference to avoid mutex contention.
pub struct VgaConsole {
    char_width: u32,
    char_height: u32,
    cols: u32,
    rows: u32,
    fg_color: Color,
    bg_color: Color,
    cursor_x: u32,
    cursor_y: u32,
    /// Pixel offset from top of framebuffer (0 for full-screen)
    y_offset: u32,
    /// Direct framebuffer pointer (byte-level, virtual address)
    fb: *mut u8,
    /// Framebuffer width in pixels
    fb_width: u32,
    /// Framebuffer height in pixels
    fb_height: u32,
    /// Bytes per scanline
    fb_stride_bytes: u32,

    // ── Scrollback support ──
    /// All non-header lines (oldest first, newest last)
    scrollback: VecDeque<String>,
    /// How many lines we've scrolled back (0 = at latest / bottom)
    scroll_pos: u32,
    /// The current line being composed (not yet pushed to scrollback)
    current_line: String,
    /// When true, new output arrived while user was scrolled back
    dirty_while_scrolled: bool,
}

// SAFETY: The framebuffer is memory-mapped I/O, accessible from any CPU core
unsafe impl Send for VgaConsole {}
unsafe impl Sync for VgaConsole {}

impl VgaConsole {
    pub fn new(char_width: u32, char_height: u32) -> Self {
        let vga = get_vga_for_sure();
        let cols = vga.width() / char_width;
        let rows = vga.height() / char_height;

        Self {
            char_width,
            char_height,
            cols,
            rows,
            fg_color: Color::LIGHT_GRAY,
            bg_color: Color::BLACK,
            cursor_x: 0,
            cursor_y: 0,
            y_offset: 0,
            fb: vga.fb_ptr() as *mut u8,
            fb_width: vga.width(),
            fb_height: vga.height(),
            fb_stride_bytes: vga.stride_pixels() * 4,
            scrollback: VecDeque::with_capacity(MAX_SCROLLBACK),
            scroll_pos: 0,
            current_line: String::with_capacity(128),
            dirty_while_scrolled: false,
        }
    }

    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    /// Direct pixel write to framebuffer — bypasses the VGA mutex entirely.
    #[inline]
    fn write_pixel(&self, x: u32, y: u32, color: Color) {
        if x < self.fb_width && y < self.fb_height {
            unsafe {
                let base = (y as usize) * (self.fb_stride_bytes as usize) + (x as usize) * 4;
                let c = color.raw();
                self.fb.add(base).write_volatile((c & 0xFF) as u8); // B
                self.fb.add(base + 1).write_volatile(((c >> 8) & 0xFF) as u8); // G
                self.fb.add(base + 2).write_volatile(((c >> 16) & 0xFF) as u8); // R
            }
        }
    }

    /// Direct rectangle fill — bypasses the VGA mutex.
    fn fill_rect_direct(&self, x: u32, y: u32, w: u32, h: u32, color: Color) {
        let x_end = (x + w).min(self.fb_width);
        let y_end = (y + h).min(self.fb_height);
        let c = color.raw();
        let b = (c & 0xFF) as u8;
        let g = ((c >> 8) & 0xFF) as u8;
        let r = ((c >> 16) & 0xFF) as u8;

        for py in y..y_end {
            unsafe {
                let row_base = (py as usize) * (self.fb_stride_bytes as usize);
                for px in x..x_end {
                    let base = row_base + (px as usize) * 4;
                    self.fb.add(base).write_volatile(b);
                    self.fb.add(base + 1).write_volatile(g);
                    self.fb.add(base + 2).write_volatile(r);
                }
            }
        }
    }

    /// Clear the entire console area (including header)
    pub fn clear(&mut self) {
        let h = self.rows * self.char_height;
        self.fill_rect_direct(0, self.y_offset, self.cols * self.char_width, h, self.bg_color);
        self.cursor_x = 0;
        self.cursor_y = HEADER_ROWS;
    }

    /// Clear only the scrollable body area (below header)
    fn clear_body(&mut self) {
        let body_y = self.y_offset + HEADER_ROWS * self.char_height;
        let body_h = (self.rows - HEADER_ROWS) * self.char_height;
        self.fill_rect_direct(0, body_y, self.cols * self.char_width, body_h, self.bg_color);
    }

    /// Draw a single character at (col, row) position using the built-in 8x16
    /// bitmap font. Uses direct framebuffer access.
    fn draw_char_at(&self, ch: char, col: u32, row: u32, fg: Color, bg: Color) {
        let px = col * self.char_width;
        let py = self.y_offset + row * self.char_height;

        // Fill character cell with background
        self.fill_rect_direct(px, py, self.char_width, self.char_height, bg);

        // Draw the character
        let idx = ch as usize;
        if idx >= 0x20 && idx <= 0x7E {
            let glyph_idx = idx - 0x20;
            if glyph_idx < FONT_8X16.len() {
                let bitmap = &FONT_8X16[glyph_idx];
                for (line, &byte) in bitmap.iter().enumerate() {
                    for bit in 0..8u32 {
                        if (byte >> (7 - bit)) & 1 != 0 {
                            self.write_pixel(px + bit, py + line as u32, fg);
                        }
                    }
                }
            }
        }
    }

    /// Render a single string line at the given screen row
    fn render_line_at(&self, line: &str, row: u32) {
        for (col, ch) in line.chars().take(self.cols as usize).enumerate() {
            self.draw_char_at(ch, col as u32, row, self.fg_color, self.bg_color);
        }
    }

    /// Re-render the entire body area from the scrollback buffer based on
    /// current scroll_pos. The body area starts after HEADER_ROWS.
    pub fn render_body(&mut self) {
        let body_rows = self.rows - HEADER_ROWS;
        let n = self.scrollback.len() as u32;

        // Compute which scrollback lines are visible
        let bottom_idx = n.saturating_sub(self.scroll_pos);
        let top_idx = bottom_idx.saturating_sub(body_rows);

        // Clear body area
        self.clear_body();

        // Render lines from top_idx to bottom_idx-1
        for i in top_idx..bottom_idx {
            let screen_row = HEADER_ROWS + (i - top_idx);
            if let Some(line) = self.scrollback.get(i as usize) {
                self.render_line_at(line, screen_row);
            }
        }
    }

    /// Scroll the scrollback view up by `n` lines
    pub fn scroll_up(&mut self, n: u32) {
        let max_scroll = self.scrollback.len() as u32;
        let body_rows = self.rows - HEADER_ROWS;
        let max_possible = max_scroll.saturating_sub(body_rows);
        let new_pos = (self.scroll_pos + n).min(max_possible);
        if new_pos != self.scroll_pos {
            self.scroll_pos = new_pos;
            self.render_body();
        }
    }

    /// Scroll the scrollback view down by `n` lines
    pub fn scroll_down(&mut self, n: u32) {
        let new_pos = self.scroll_pos.saturating_sub(n);
        if new_pos != self.scroll_pos {
            self.scroll_pos = new_pos;
            self.render_body();
            // If we're back at the bottom, discard the dirty flag
            if self.scroll_pos == 0 {
                self.dirty_while_scrolled = false;
            }
        }
    }

    /// Push a completed line to the scrollback buffer
    fn push_line(&mut self, line: String) {
        if self.scrollback.len() >= MAX_SCROLLBACK {
            self.scrollback.pop_front();
        }
        self.scrollback.push_back(line);
    }

    /// Flush the current accumulated line to the scrollback buffer
    fn flush_current_line(&mut self) {
        if !self.current_line.is_empty() {
            let line = core::mem::replace(
                &mut self.current_line,
                String::with_capacity(128),
            );
            self.push_line(line);
        }
    }

    /// Write a single character at current cursor position.
    ///
    /// In live mode (`scroll_pos == 0`) the character is drawn directly to the
    /// framebuffer and the built-in scroll machinery handles text scrolling.
    /// Lines that scroll off-screen are saved into `scrollback`.
    ///
    /// In scrollback mode (`scroll_pos > 0`) output is buffered into
    /// `scrollback` without touching the framebuffer, so the user can keep
    /// reading old content.  The `dirty_while_scrolled` flag is set to remind
    /// the user that new output arrived.
    pub fn write_char(&mut self, ch: char) {
        // ── 1. Accumulate into current_line (always) ──
        match ch {
            '\n' => {
                self.flush_current_line();
            }
            '\r' => {
                // Discard any partial line (carriage return in raw terminal)
                self.current_line.clear();
            }
            '\x08' => {
                // Backspace: remove last char from current_line
                self.current_line.pop();
            }
            '\t' => {
                let spaces = 4 - (self.cursor_x % 4);
                for _ in 0..spaces {
                    self.current_line.push(' ');
                }
            }
            _ => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    self.current_line.push(ch);
                }
            }
        }

        // ── 2. If scrolled back, buffer only ──
        if self.scroll_pos > 0 {
            self.dirty_while_scrolled = true;
            // Keep cursor tracking up to date
            match ch {
                '\n' => {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                }
                '\r' => {
                    self.cursor_x = 0;
                }
                '\x08' => {
                    if self.cursor_x > 0 {
                        self.cursor_x -= 1;
                    } else if self.cursor_y > HEADER_ROWS {
                        self.cursor_y -= 1;
                        self.cursor_x = self.cols - 1;
                    }
                }
                '\t' => {
                    self.cursor_x = ((self.cursor_x / 4) + 1) * 4;
                }
                _ => {
                    if ch.is_ascii_graphic() || ch == ' ' {
                        self.cursor_x += 1;
                    }
                }
            }
            if self.cursor_x >= self.cols {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
            return;
        }

        // ── 3. Live mode: draw to framebuffer (original behavior) ──
        match ch {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
            '\r' => {
                self.cursor_x = 0;
            }
            '\x08' => {
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > HEADER_ROWS {
                    self.cursor_y -= 1;
                    self.cursor_x = self.cols - 1;
                }
                // Erase the character cell (only in live mode)
                self.draw_char_at(
                    ' ', self.cursor_x, self.cursor_y,
                    self.fg_color, self.bg_color,
                );
            }
            '\t' => {
                let spaces = 4 - (self.cursor_x % 4);
                for _ in 0..spaces {
                    self.draw_char_at(
                        ' ', self.cursor_x, self.cursor_y,
                        self.fg_color, self.bg_color,
                    );
                    self.cursor_x += 1;
                }
                return; // skip late wrap check inside the loop body
            }
            _ => {
                if ch.is_ascii_graphic() || ch == ' ' {
                    self.draw_char_at(
                        ch, self.cursor_x, self.cursor_y,
                        self.fg_color, self.bg_color,
                    );
                    self.cursor_x += 1;
                }
            }
        }

        // ── line wrapping ──
        if self.cursor_x >= self.cols {
            self.cursor_x = 0;
            self.cursor_y += 1;
        }

        // ── scrolling (body area only) ──
        if self.cursor_y >= self.rows {
            // Scroll the body area up by one line
            let body_start = self.y_offset + HEADER_ROWS * self.char_height;
            let body_end = self.y_offset + self.rows * self.char_height;
            let body_h = body_end - body_start;
            let line_h = self.char_height;
            let viewport_w = (self.cols * self.char_width) as usize;
            let stride = (self.fb_stride_bytes / 4) as usize;

            // Copy body rows up by one line
            for py in 0..(body_h - line_h) {
                let src_y = body_start + py + line_h;
                let dst_y = body_start + py;
                unsafe {
                    core::ptr::copy(
                        (self.fb as *mut u32).add((src_y as usize) * stride),
                        (self.fb as *mut u32).add((dst_y as usize) * stride),
                        viewport_w,
                    );
                }
            }
            // Clear the last line of the body
            let last_y = body_end - line_h;
            self.fill_rect_direct(
                0,
                last_y,
                self.cols * self.char_width,
                self.char_height,
                self.bg_color,
            );
            self.cursor_y = self.rows - 1;
        }
    }

    /// Write a string using direct framebuffer access.
    pub fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.write_char(ch);
        }
    }
}

impl fmt::Write for VgaConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// Global VGA console
once_mutex!(pub VGA_CONSOLE: VgaConsole);
guard_access_fn!(pub get_vga_console(VGA_CONSOLE: VgaConsole));

/// Acquire the VGA console lock, spinning until it becomes available.
/// Returns `None` if the console hasn't been initialized yet.
/// Used by `print_internal` — safe because it runs with interrupts disabled.
pub fn get_vga_console_spin<'a>() -> Option<spin::MutexGuard<'a, VgaConsole>> {
    let m = VGA_CONSOLE.get()?;
    loop {
        if let Some(guard) = m.try_lock() {
            return Some(guard);
        }
        core::hint::spin_loop();
    }
}

/// Try to write formatted output to the VGA console (best-effort).
/// Uses try_lock — safe from interrupt context, silently skips if console is
/// busy. Used by the logger to show log messages on the VGA display when
/// possible without risking deadlock.
pub fn try_write_to_vga(args: core::fmt::Arguments) {
    if let Some(mut console) = get_vga_console() {
        use core::fmt::Write;
        let _ = console.write_fmt(args);
    }
}

/// Scroll the VGA console up by 1 line (for up-arrow key).
/// Safe to call from interrupt context (uses try_lock).
pub fn scroll_console_up() {
    if let Some(mut console) = get_vga_console() {
        console.scroll_up(1);
    }
}

/// Scroll the VGA console down by 1 line (for down-arrow key).
/// Safe to call from interrupt context (uses try_lock).
pub fn scroll_console_down() {
    if let Some(mut console) = get_vga_console() {
        console.scroll_down(1);
    }
}

/// Scroll the VGA console up by half a screen (for page-up key).
/// Safe to call from interrupt context (uses try_lock).
pub fn scroll_console_up_pages(n: u32) {
    if let Some(mut console) = get_vga_console() {
        let body_rows = console.rows.saturating_sub(HEADER_ROWS);
        let amount = n * (body_rows / 2).max(1);
        console.scroll_up(amount);
    }
}

/// Scroll the VGA console down by half a screen (for page-down key).
/// Safe to call from interrupt context (uses try_lock).
pub fn scroll_console_down_pages(n: u32) {
    if let Some(mut console) = get_vga_console() {
        let body_rows = console.rows.saturating_sub(HEADER_ROWS);
        let amount = n * (body_rows / 2).max(1);
        console.scroll_down(amount);
    }
}

/// Initialize the VGA console in split-screen mode:
/// Upper portion — animated clock, drawn by the `clock_demo` process.
/// Lower portion — text console for shell + log output, with a fixed
///   header showing the YSOS banner and a scrollable body area.
pub fn init_vga_console_for_shell() -> u32 {
    let char_width = 8u32;
    let char_height = 16u32;

    let (vga_width, vga_height) = {
        let vga = get_vga_for_sure();
        (vga.width(), vga.height())
    };

    let total_rows = vga_height / char_height;
    // Clock gets ~45%, text gets ~55% (close to half-and-half)
    let clock_rows = total_rows * 9 / 20;
    let clock_height = clock_rows * char_height;
    let separator_y = clock_height;

    // Create console for the lower portion
    let mut console = VgaConsole::new(char_width, char_height);
    console.rows = total_rows - clock_rows;
    console.cols = vga_width / char_width;
    console.y_offset = separator_y;
    // Cursor starts after the header
    console.cursor_x = 0;
    console.cursor_y = HEADER_ROWS;
    // Green-on-dark for a terminal feel
    console.set_colors(Color::new(180, 200, 180), Color::new(12, 14, 20));

    {
        let vga = get_vga_for_sure();
        // Upper area: dark bg (clock will draw over this)
        vga.fill_rect(0, 0, vga_width, separator_y, Color::new(16, 18, 28));
        // Lower area: terminal bg
        vga.fill_rect(
            0,
            separator_y,
            vga_width,
            vga_height - separator_y,
            Color::new(12, 14, 20),
        );
        // Separator line
        vga.fill_rect(0, separator_y, vga_width, 1, Color::new(0, 140, 100));

        // Draw header title in the fixed header area
        // We use the VGA directly for the header since it's fixed
        let header_fg = Color::new(0, 220, 160);
        let header_bg = Color::new(12, 14, 20);

        // Helper lambda to draw a header string at a given row
        let draw_header_text = |text: &str, row: u32, fg: Color, bg: Color| {
            let py = separator_y + row * char_height;
            for (col, ch) in text.chars().enumerate() {
                let px = col as u32 * char_width;
                vga.fill_rect(px, py, char_width, char_height, bg);
                let idx = ch as usize;
                if idx >= 0x20 && idx <= 0x7E {
                    let glyph_idx = idx - 0x20;
                    if glyph_idx < FONT_8X16.len() {
                        let bitmap = &FONT_8X16[glyph_idx];
                        for (line, &byte) in bitmap.iter().enumerate() {
                            for bit in 0..8u32 {
                                if (byte >> (7 - bit)) & 1 != 0 {
                                    if px + bit < vga_width && (py + (line as u32)) < vga_height {
                                        unsafe {
                                            let fb = vga.fb_ptr() as *mut u8;
                                            let sb = vga.stride_pixels() * 4;
                                            let off = ((py + line as u32) as usize)
                                                * (sb as usize)
                                                + ((px + bit) as usize) * 4;
                                            let c = fg.raw();
                                            fb.add(off).write_volatile((c & 0xFF) as u8);
                                            fb.add(off + 1)
                                                .write_volatile(((c >> 8) & 0xFF) as u8);
                                            fb.add(off + 2)
                                                .write_volatile(((c >> 16) & 0xFF) as u8);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        // Draw YSOS banner in the fixed header
        let ascii = crate::get_ascii_header();
        for (row, line) in ascii.lines().enumerate() {
            if row < HEADER_ROWS as usize {
                draw_header_text(line, row as u32, header_fg, header_bg);
            }
        }
    }

    init_VGA_CONSOLE(console);

    // Print welcome message in the body area (below the fixed header)
    if let Some(mut cons) = get_vga_console() {
        use core::fmt::Write;
        let _ = write!(cons, "VGA Display Initialized.\n");
        let _ = write!(cons, "Use Up/Down/PgUp/PgDn to scroll.\n\n");
    }

    info!(
        "VGA Console: {}x{} chars, {} header rows, {} body rows, clock_height={}",
        vga_width / char_width,
        total_rows - clock_rows,
        HEADER_ROWS,
        total_rows - clock_rows - HEADER_ROWS,
        clock_height,
    );

    clock_height
}
