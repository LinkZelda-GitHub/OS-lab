use crate::memory::gdt;
use crate::proc::ProcessContext;
use crate::as_handler;

use super::consts::*;
use core::sync::atomic::AtomicU64;
use core::fmt::{Debug, Error, Formatter};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use core::sync::atomic::{AtomicUsize, Ordering};

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Timer as u8]
        .set_handler_fn(clock_handler)
        .set_stack_index(gdt::CLOCK_IST_INDEX);
}

pub extern "C" fn clock(mut context: ProcessContext) {
    super::ack();

    crate::proc::switch(&mut context);
}

as_handler!(clock);