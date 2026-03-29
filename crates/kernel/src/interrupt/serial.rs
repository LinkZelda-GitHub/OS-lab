use super::consts::*;
use uefi::proto::console::text::Input;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::drivers::input::{push_key, try_pop_key};
use crate::drivers::uart16550;

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Serial0 as u8]
        .set_handler_fn(serial_handler);
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
    if let Some(byte) = serial_port.receive() {
        push_key(byte);
    }
}