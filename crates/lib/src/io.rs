use alloc::{
    string::{String, ToString},
    vec,
};

use crate::*;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    fn new() -> Self {
        Self
    }

    pub fn read_line(&self) -> String {
        // FIXED: allocate string
        let mut buf = String::new();
        let mut char_buf = [0u8; 1];

        // FIXED: read from input buffer
        //       - maybe char by char?
        // FIXED: handle backspace / enter...
        // FIXED: return string
        loop {
            // 一次读取 1 字节
            if crate::sys_read(0, &mut char_buf) == Some(1) {
                let c = char_buf[0];

                if c == b'\n' || c == b'\r' {
                    crate::sys_write(1, b"\n");
                    break;
                } else if c == 0x08 || c == 0x7f {
                    // Backspace or Delete
                    if buf.pop().is_some() {
                        // 退格表现：光标左移 + 输出空格覆盖 + 光标再次左移
                        crate::sys_write(1, b"\x08 \x08");
                    }
                } else if c.is_ascii() && c >= 32 {
                    buf.push(c as char);
                    crate::sys_write(1, &char_buf); // 回显到屏幕
                }
            } else {
                // 没有输入时稍微等待一下，避免忙等待过热
                core::hint::spin_loop();
            }
        }
        buf
    }
}

impl Stdout {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(1, s.as_bytes());
    }
}

impl Stderr {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(2, s.as_bytes());
    }
}

pub fn stdin() -> Stdin {
    Stdin::new()
}

pub fn stdout() -> Stdout {
    Stdout::new()
}

pub fn stderr() -> Stderr {
    Stderr::new()
}
