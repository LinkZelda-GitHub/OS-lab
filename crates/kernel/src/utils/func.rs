use core::f64::consts::PI;

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, iso_8859_1::FONT_6X10},
    prelude::*,
    primitives::{Circle, Line, Primitive, PrimitiveStyle, Rectangle},
    text::{Baseline, Text, TextStyleBuilder},
};

use crate::drivers::vga::{Color, VgaDisplay, get_vga};

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

    // Colors
    let bg_color = Color::new(20, 22, 34);
    let border_color = Color::new(80, 90, 120);
    let major_tick = Color::new(200, 210, 230);
    let minor_tick = Color::new(50, 60, 80);
    let title_color = Color::new(0, 180, 140);

    let mut total_seconds: u32 = 0;

    loop {
        total_seconds = total_seconds.wrapping_add(1);

        if let Some(vga) = get_vga() {
            // Clear clock area with dark background
            vga.fill_rect(0, 0, tw, ch, bg_color);

            let mut display = VgaDisplay::new(&vga);

            // ── Title bar ──
            let title_style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(title_color)
                .build();
            let _ = Text::with_text_style(
                "YatSenOS Clock",
                Point::new(8, 6),
                title_style,
                TextStyleBuilder::new().baseline(Baseline::Top).build(),
            );

            // ── Clock face border (double circle) ──
            let outer_style = PrimitiveStyle::with_stroke(border_color, 2);
            let _ = Circle::new(Point::new(cx, cy), radius as u32 + 2)
                .into_styled(outer_style)
                .draw(&mut display);
            let inner_style = PrimitiveStyle::with_stroke(Color::new(60, 70, 95), 1);
            let _ = Circle::new(Point::new(cx, cy), radius as u32)
                .into_styled(inner_style)
                .draw(&mut display);

            // ── Hour markers ──
            for h in 0..12 {
                let angle = (h as f64) * (PI / 6.0) - PI / 2.0;
                let (sin_a, cos_a) = (libm::sin(angle), libm::cos(angle));
                let outer_r = (radius - 7) as f64;
                let inner_r = if h % 3 == 0 {
                    (radius - 18) as f64
                } else {
                    (radius - 12) as f64
                };
                let x1 = (cx as f64 + inner_r * cos_a) as i32;
                let y1 = (cy as f64 + inner_r * sin_a) as i32;
                let x2 = (cx as f64 + outer_r * cos_a) as i32;
                let y2 = (cy as f64 + outer_r * sin_a) as i32;

                let marker_color = if h % 3 == 0 {
                    major_tick
                } else {
                    minor_tick
                };
                let thickness: u32 = if h % 3 == 0 { 2 } else { 1 };
                let _ = Line::new(Point::new(x1, y1), Point::new(x2, y2))
                    .into_styled(PrimitiveStyle::with_stroke(marker_color, thickness))
                    .draw(&mut display);
            }

            // ── Hour numbers (12, 3, 6, 9) ──
            let num_style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(major_tick)
                .build();
            let hour_nums = [12u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
            for (i, &num) in hour_nums.iter().enumerate() {
                let angle = (i as f64) * (PI / 6.0) - PI / 2.0;
                let num_r = (radius - 28) as f64;
                let nx = (cx as f64 + num_r * libm::cos(angle) - 3.0) as i32;
                let ny = (cy as f64 + num_r * libm::sin(angle) - 5.0) as i32;
                let num_str = alloc::format!("{}", num);
                let _ = Text::with_text_style(
                    &num_str,
                    Point::new(nx, ny),
                    num_style,
                    TextStyleBuilder::new().baseline(Baseline::Top).build(),
                )
                .draw(&mut display);
            }

            // ── Draw hands ──
            let sec_angle = (total_seconds % 60) as f64 * (PI / 30.0) - PI / 2.0;
            let min_angle = ((total_seconds / 60) % 60) as f64 * (PI / 30.0)
                + (total_seconds % 60) as f64 * (PI / 1800.0)
                - PI / 2.0;
            let hour_angle = ((total_seconds / 3600) % 12) as f64 * (PI / 6.0)
                + ((total_seconds / 60) % 60) as f64 * (PI / 360.0)
                - PI / 2.0;

            // Second hand: bright red
            let sl = (radius - 12) as f64;
            let sx = (cx as f64 + sl * libm::cos(sec_angle)) as i32;
            let sy = (cy as f64 + sl * libm::sin(sec_angle)) as i32;
            let _ = Line::new(Point::new(cx, cy), Point::new(sx, sy))
                .into_styled(PrimitiveStyle::with_stroke(Color::RED, 1))
                .draw(&mut display);

            // Second hand tail
            let stl = (radius / 4) as f64;
            let stx = (cx as f64 - stl * libm::cos(sec_angle)) as i32;
            let sty = (cy as f64 - stl * libm::sin(sec_angle)) as i32;
            let _ = Line::new(Point::new(cx, cy), Point::new(stx, sty))
                .into_styled(PrimitiveStyle::with_stroke(Color::RED, 1))
                .draw(&mut display);

            // Minute hand: light blue, thick
            let ml = (radius - 24) as f64;
            let mx = (cx as f64 + ml * libm::cos(min_angle)) as i32;
            let my = (cy as f64 + ml * libm::sin(min_angle)) as i32;
            for off in -1..=1i32 {
                let _ = Line::new(
                    Point::new(cx + off, cy),
                    Point::new(mx + off, my),
                )
                .into_styled(PrimitiveStyle::with_stroke(Color::new(150, 200, 255), 1))
                .draw(&mut display);
            }

            // Hour hand: white, thickest
            let hl = (radius - 40) as f64;
            let hx = (cx as f64 + hl * libm::cos(hour_angle)) as i32;
            let hy = (cy as f64 + hl * libm::sin(hour_angle)) as i32;
            for off in -1..=1i32 {
                let _ = Line::new(
                    Point::new(cx + off, cy),
                    Point::new(hx + off, hy),
                )
                .into_styled(PrimitiveStyle::with_stroke(Color::WHITE, 1))
                .draw(&mut display);
            }

            // ── Center cap ──
            let _ = Circle::new(Point::new(cx - 3, cy - 3), 6)
                .into_styled(PrimitiveStyle::with_fill(Color::new(0, 200, 120)))
                .draw(&mut display);
            let _ = Circle::new(Point::new(cx - 1, cy - 1), 2)
                .into_styled(PrimitiveStyle::with_fill(Color::BLACK))
                .draw(&mut display);

            // ── Digital time display ──
            let time_str = alloc::format!(
                "{:02}:{:02}:{:02}",
                (total_seconds / 3600) % 24,
                (total_seconds / 60) % 60,
                total_seconds % 60
            );
            let dl_style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(Color::new(240, 240, 255))
                .build();
            let _ = Text::with_text_style(
                &time_str,
                Point::new(cx - 15, cy + radius + 14),
                dl_style,
                TextStyleBuilder::new().baseline(Baseline::Top).build(),
            );

            // Separator bar at the bottom of the clock area
            vga.fill_rect(0, ch - 1, tw, 1, Color::new(0, 140, 100));
        }

        // Sleep ~1 second using hlt in a loop (no RTC timer, approximate)
        for _ in 0..60_000_000 {
            core::hint::spin_loop();
        }
    }
}
