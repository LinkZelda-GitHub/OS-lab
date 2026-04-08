use super::consts::*;
use uefi::proto::console::text::Input;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::drivers::input::{push_key, try_pop_key};
use crate::drivers::uart16550;

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Serial0 as u8]
        .set_handler_fn(serial_handler);

     // 严谨的 UART 中断开启流程：
    let mut lcr = x86_64::instructions::port::Port::<u8>::new(0x3F8 + 3);
    let mut ier = x86_64::instructions::port::Port::<u8>::new(0x3F8 + 1);
    
    // 1. 强制设置 8位数据长度，并剥夺 DLAB 位（将 DLAB 强行置 0，LCR = 0x03）
    lcr.write(0x03);
    
    // 2. 现在安全了，偏移 +1 绝对是 IER 寄存器，写入 0x01 开启接收中断
    ier.write(0x01);
}

pub extern "x86-interrupt" fn serial_handler(_st: InterruptStackFrame) {
    receive();
    super::ack();
}

/// Receive character from uart 16550
/// Should be called on every interrupt
fn receive() {
    // FIXED: receive character from uart 16550, put it into INPUT_BUFFER
    let mut serial_port = uart16550::SerialPort::new(0x3F8); // COM1 基地址
    
    while let Some(byte) = serial_port.receive() {
        push_key(byte);
    }

    unsafe {
        let _ = x86_64::instructions::port::Port::<u8>::new(0x3F8 + 5).read(); // 读取 LSR
        let _ = x86_64::instructions::port::Port::<u8>::new(0x3F8 + 6).read(); // 读取 MSR
    }
}