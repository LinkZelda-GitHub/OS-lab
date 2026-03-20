use core::fmt::Write;

use lazy_static::lazy_static;
use crossbeam_queue::ArrayQueue;
use log::warn;
use uefi::proto::console::serial::Serial;
use super::uart16550::SerialPort;
use alloc::string::String;

// 定义输入缓冲区存储的数据类型（从串口接收的原始字节）
type Key = u8;

// 使用无锁队列作为输入缓冲区，容量 128
lazy_static! {
    static ref INPUT_BUF: ArrayQueue<Key> = ArrayQueue::new(128);
}

/// 将接收到的字符放入输入缓冲区（由中断处理程序调用）
#[inline]
pub fn push_key(key: Key) {
    if INPUT_BUF.push(key).is_err() {
        warn!("Input buffer is full. Dropping key '{:?}'", key);
    }
}

/// 非阻塞地从缓冲区取出一个字符
#[inline]
pub fn try_pop_key() -> Option<Key> {
    INPUT_BUF.pop()
}

/// 阻塞地从缓冲区取出一个字符（循环等待直到有数据）
pub fn pop_key() -> Key {
    loop {
        if let Some(key) = try_pop_key() {
            return key;
        }
    }
}

/// 阻塞地读取一行输入，并实时回显，支持退格删除。
/// 遇到换行符 `\n` 或回车符 `\r` 时结束，返回用户输入的字符串（不含换行符）。
pub fn get_line() -> String {
    let mut serial_port = SerialPort::new(0x3F8); // COM1 基地址
    let mut line = String::with_capacity(128);
    loop {
        let ch = pop_key();
        match ch {
            b'\n' | b'\r' => {
                // 换行/回车：打印换行并结束读取
                write!(serial_port, "\n").unwrap();
                break;
            }
            0x08 | 0x7F => { // 退格（Backspace / Delete）
                if !line.is_empty() {
                    line.pop();
                    // 调用串口驱动提供的退格函数，删除屏幕上最后一个字符
                    write!(serial_port, "\x08 \x08").unwrap(); // 退格 + 空格 + 退格，覆盖最后一个字符
                }
            }
            _ => {
                // 仅处理可打印字符（ASCII 32-126，即空格到波浪号）
                if ch >= 0x20 && ch <= 0x7E {
                    line.push(ch as char);
                    write!(serial_port, "{}", ch as char).unwrap();
                }
                // 忽略其他控制字符（如制表符、铃声等）
            }
        }
    }
    line
}