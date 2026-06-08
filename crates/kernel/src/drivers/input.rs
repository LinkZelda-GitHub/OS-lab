use alloc::string::String;
use core::fmt::Write;

use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;

use super::uart16550::SerialPort;

// 定义输入缓冲区存储的数据类型（从串口接收的原始字节）
type Key = u8;

// 使用无锁队列作为输入缓冲区，容量 128
lazy_static! {
    static ref INPUT_BUF: ArrayQueue<Key> = ArrayQueue::new(128);
}

/// 将接收到的字符放入输入缓冲区（由中断处理程序调用）
#[inline]
pub fn push_key(key: Key) {
    let _ = INPUT_BUF.push(key);
}

/// 非阻塞地从缓冲区取出一个字符
#[inline]
pub fn try_pop_key() -> Option<Key> {
    INPUT_BUF.pop()
}

/// 阻塞地从缓冲区取出一个字符（循环等待直到有数据）
pub fn pop_key() -> Key {
    loop {
        x86_64::instructions::interrupts::disable();

        if let Some(key) = try_pop_key() {
            x86_64::instructions::interrupts::enable();
            return key;
        }
        // 如果没有字符，让 CPU 休眠，直到下一次任意硬件中断唤醒它
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

/// Echo a character to the VGA console (best-effort, uses try_lock)
fn echo_to_vga(ch: char) {
    // Spin-lock the VGA console so we never drop a visual echo
    if let Some(mut console) = super::vga_text::get_vga_console() {
        use core::fmt::Write;
        let _ = write!(console, "{}", ch);
    }
}

/// 阻塞地读取一行输入，并实时回显，支持退格删除。
/// 遇到换行符 `\n` 或回车符 `\r` 时结束，返回用户输入的字符串（不含换行符）。
/// Echoes characters to both serial port and VGA console.
pub fn get_line() -> String {
    let mut serial_port = SerialPort::new(0x3F8); // COM1 基地址
    let mut line = String::with_capacity(128);

    loop {
        let ch = pop_key();
        match ch {
            b'\n' | b'\r' => {
                // 换行/回车：打印换行并结束读取
                write!(serial_port, "\n").unwrap();
                echo_to_vga('\n');
                break;
            }
            0x08 | 0x7F => {
                // 退格（Backspace / Delete）
                if !line.is_empty() {
                    line.pop();
                    // 调用串口驱动提供的退格函数，删除屏幕上最后一个字符
                    write!(serial_port, "\x08 \x08").unwrap(); // 退格 + 空格 + 退格，覆盖最后一个字符
                    // VGA echo: backspace
                    echo_to_vga('\x08');
                }
            }
            _ => {
                // 仅处理可打印字符（ASCII 32-126，即空格到波浪号）
                if ch >= 0x20 && ch <= 0x7E {
                    line.push(ch as char);
                    write!(serial_port, "{}", ch as char).unwrap();
                    echo_to_vga(ch as char);
                }
                // 忽略其他控制字符（如制表符、铃声等）
            }
        }
    }
    line
}
