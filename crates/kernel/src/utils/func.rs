use core::f64::consts::PI;

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, iso_8859_1::FONT_6X10},
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};

use crate::drivers::vga::{Color, VgaDisplay, VgaDriver, get_vga};

pub fn test() -> ! {
    let mut count = 0;
    let id;
    if let Some(id_env) = crate::proc::env("id") {
        id = id_env
    } else {
        id = "unknown".into()
    }
    loop {
        count += 1;
        if count == 1000 {
            count = 0;
            print!("\r{:-6} => Tick!", id);
        }
        x86_64::instructions::hlt();
    }
}

#[inline(never)]
fn huge_stack() {
    println!("Huge stack testing...");
    let mut stack = [0u64; 0x1000];
    for (idx, item) in stack.iter_mut().enumerate() {
        *item = idx as u64;
    }
    for i in 0..stack.len() / 256 {
        println!("{:#05x} == {:#05x}", i * 256, stack[i * 256]);
    }
}

pub fn stack_test() -> ! {
    huge_stack();
    crate::proc::print_process_list();
    crate::proc::process_exit(0)
}

/// Get current time as milliseconds since midnight using UEFI runtime.
fn get_time_ms() -> usize {
    let t = uefi::runtime::get_time().expect("Failed to get UEFI time");
    t.second() as usize * 1000
        + t.minute() as usize * 60 * 1000
        + t.hour() as usize * 3600 * 1000
        + (t.nanosecond() / 1_000_000) as usize
}

/// Draw a thick line by plotting individual pixels along the line segment,
/// using a filled circle (stamp) at each step so the line is always visible.
fn draw_thick_line(vga: &VgaDriver, x1: i32, y1: i32, x2: i32, y2: i32, color: Color, width: i32) {
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x1;
    let mut y = y1;
    let half = width / 2;

    loop {
        // Draw a small filled square centered at (x, y)
        for wy in -half..=half {
            for wx in -half..=half {
                vga.draw_pixel((x + wx) as u32, (y + wy) as u32, color);
            }
        }
        if x == x2 && y == y2 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            if x == x2 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y2 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

/// Draw an animated analog clock on the upper portion of the VGA display.
/// Uses embedded-graphics primitives for clean rendering.
///
/// The clock runs as a background kernel thread and draws into the top
/// portion of the framebuffer (above the shell's text area).
pub fn clock_demo() {
    // Read the clock area height from the environment (set by kernel_main)
    let clock_height: u32 = crate::proc::env("CLOCK_HEIGHT")
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);

    let tw: u32;
    let ch: u32;
    {
        let vga = match get_vga() {
            Some(v) => v,
            None => return,
        };
        tw = vga.width();
        ch = clock_height.min(vga.height());
    }

    let cx = (tw / 2) as i32;
    let cy = (ch / 2) as i32;
    let radius = (ch.min(tw) as i32 / 2).saturating_sub(16);

    let bg_color = Color::new(20, 22, 34);

    let mut total_seconds: u32 = 0;

    loop {
        total_seconds = total_seconds.wrapping_add(1);

        if let Some(vga) = get_vga() {
            // Clear clock area with dark background
            vga.fill_rect(0, 0, tw, ch, bg_color);

            // ── Tick marks (every 5 seconds, 60 positions) ──
            for s in 0..60 {
                let angle = (s as f64) * (PI / 30.0) - PI / 2.0;
                let outer_r = (radius - 8) as f64;
                let inner_r = if s % 5 == 0 {
                    (radius - 22) as f64 // long tick at hour positions
                } else {
                    (radius - 14) as f64 // short tick every 5 sec
                };
                let x1 = (cx as f64 + inner_r * libm::cos(angle)) as i32;
                let y1 = (cy as f64 + inner_r * libm::sin(angle)) as i32;
                let x2 = (cx as f64 + outer_r * libm::cos(angle)) as i32;
                let y2 = (cy as f64 + outer_r * libm::sin(angle)) as i32;
                let (color, w): (Color, i32) = if s % 5 == 0 {
                    (Color::new(200, 210, 230), 2)
                } else {
                    (Color::new(50, 60, 80), 1)
                };
                draw_thick_line(&vga, x1, y1, x2, y2, color, w);
            }

            // ── Hour numbers (1–12) ──
            {
                let mut display = VgaDisplay::new(&vga);
                let num_style = MonoTextStyleBuilder::new()
                    .font(&FONT_6X10)
                    .text_color(Color::new(200, 210, 230))
                    .build();
                let nums = [12u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
                for (i, &num) in nums.iter().enumerate() {
                    let angle = (i as f64) * (PI / 6.0) - PI / 2.0;
                    let r = (radius - 30) as f64;
                    let nx = (cx as f64 + r * libm::cos(angle) - 3.0) as i32;
                    let ny = (cy as f64 + r * libm::sin(angle) - 5.0) as i32;
                    let _ = Text::with_text_style(
                        &alloc::format!("{}", num),
                        Point::new(nx, ny),
                        num_style,
                        TextStyleBuilder::new().baseline(Baseline::Top).build(),
                    )
                    .draw(&mut display);
                }
            }

            // ── Draw hands ──
            let sec_angle = (total_seconds % 60) as f64 * (PI / 30.0) - PI / 2.0;
            let min_angle = ((total_seconds / 60) % 60) as f64 * (PI / 30.0)
                + (total_seconds % 60) as f64 * (PI / 1800.0)
                - PI / 2.0;
            let hour_angle = ((total_seconds / 3600) % 12) as f64 * (PI / 6.0)
                + ((total_seconds / 60) % 60) as f64 * (PI / 360.0)
                - PI / 2.0;

            // ── Hour hand: white ──
            {
                let hl = (radius - 42) as f64;
                let hx = (cx as f64 + hl * libm::cos(hour_angle)) as i32;
                let hy = (cy as f64 + hl * libm::sin(hour_angle)) as i32;
                draw_thick_line(&vga, cx, cy, hx, hy, Color::WHITE, 3);
            }

            // ── Minute hand: light blue, thick ──
            {
                let ml = (radius - 26) as f64;
                let mx = (cx as f64 + ml * libm::cos(min_angle)) as i32;
                let my = (cy as f64 + ml * libm::sin(min_angle)) as i32;
                draw_thick_line(&vga, cx, cy, mx, my, Color::new(150, 210, 255), 3);
            }

            // ── Second hand: orange-red ──
            {
                let sl = (radius - 4) as f64;
                let sx = (cx as f64 + sl * libm::cos(sec_angle)) as i32;
                let sy = (cy as f64 + sl * libm::sin(sec_angle)) as i32;
                draw_thick_line(&vga, cx, cy, sx, sy, Color::new(255, 80, 40), 2);
            }
        }

        // Wait 1 second using UEFI time.
        {
            let start = get_time_ms();
            loop {
                if get_time_ms().wrapping_sub(start) >= 1000 {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }
}
