//! Keyboard (IRQ1) interrupt handler

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use super::consts::*;
use crate::drivers::keyboard::get_ps2_keyboard_for_sure;

/// Register the keyboard interrupt handler in the IDT
pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Keyboard as u8].set_handler_fn(keyboard_handler);
}

/// Keyboard interrupt handler
pub extern "x86-interrupt" fn keyboard_handler(_st: InterruptStackFrame) {
    // Read the scancode from port 0x60
    let scancode: u8 = unsafe { x86_64::instructions::port::Port::new(0x60).read() };

    // Process the scancode
    let mut kb = get_ps2_keyboard_for_sure();
    kb.handle_scancode(scancode);

    // Acknowledge the interrupt
    super::ack();
}
