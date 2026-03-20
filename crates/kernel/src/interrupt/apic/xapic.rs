use core::{
    fmt::{Debug, Error, Formatter},
    ptr::{read_volatile, write_volatile},
};

use bit_field::BitField;
use x86::{cpuid::CpuId, segmentation::BuildDescriptor};

use super::LocalApic;

use crate::memory::physical_to_virtual;
use crate::interrupt::consts::{Interrupts, Irq};

/// Default physical address of xAPIC
pub const LAPIC_ADDR: u64 = 0xFEE00000;

pub struct XApic {
    addr: u64,
}

impl XApic {
    pub unsafe fn new(addr: u64) -> Self {
        XApic { addr }
    }

    unsafe fn read(&self, reg: u32) -> u32 {
        unsafe { read_volatile((self.addr + reg as u64) as *const u32) }
    }

    unsafe fn write(&mut self, reg: u32, value: u32) {
        unsafe {
            write_volatile((self.addr + reg as u64) as *mut u32, value);
            self.read(0x20);
        }
    }
}

impl LocalApic for XApic {
    /// If this type APIC is supported
    fn support() -> bool {
        // FIXED: Check CPUID to see if xAPIC is supported.
        CpuId::new().get_feature_info().map(
            |f| f.has_apic()
        ).unwrap_or(false)
    }

    /// Initialize the xAPIC for the current CPU.
    fn cpu_init(&mut self) {
        unsafe {
            // FIXED: Enable local APIC; set spurious interrupt vector.
            let mut spiv = self.read(0xF0);
            spiv |= 1<<8; // set EN bit
            // clear and set Vector
            spiv &= !(0xFF);
            spiv |= Interrupts::IrqBase as u32 + Irq::Spurious as u32;
            self.write(0xF0, spiv);

            // FIXED: The timer repeatedly counts down at bus frequency

            let mut lvt_timer = self.read(0x320);
            // clear and set Vector
            lvt_timer &= !(0xFF);
            lvt_timer |= Interrupts::IrqBase as u32 + Irq::Timer as u32;
            lvt_timer &= !(1 << 16); // clear Mask
            lvt_timer |= 1 << 17; // set Timer Periodic Mode
            self.write(0x320, lvt_timer);

            self.write(0x3E0, 0b1011); // set Timer Divide to 1
            // self.write(0x3E0, 0b0000); // Test: set Timer Divide to 2
            // self.write(0x3E0, 0b1000); // Test set Timer Divide to 4
            self.write(0x380, 0x20000); // set initial count to 0x20000

            // FIXED: Disable logical interrupt lines (LINT0, LINT1)
            self.write(0x350, 1 << 16); // set Mask LINT0
            self.write(0x360, 1 << 16); // set Mask LINT1

            // FIXED: Disable performance counter overflow interrupts (PCINT)
            self.write(0x340, 1 << 16); // set Mask PCINT

            // FIXED: Map error interrupt to IRQ_ERROR.
            self.write(0x370, Interrupts::IrqBase as u32 + Irq::Error as u32);

            // FIXED: Clear error status register (requires back-to-back
            // writes).
            self.write(0x280, 0); // clear error status register
            self.write(0x280, 0); // clear error status register Again

            // FIXED: Ack any outstanding interrupts.   
            self.write(0x0b0, 0); // write EOI as zero

            // FIXED: Send an Init Level De-Assert to synchronise arbitration
            // ID's.
            self.write(0x310, 0); // set ICR 0x310
            const BCAST: u32 = 1 << 19;
            const INIT: u32 = 5 << 8;
            const TMLV: u32 = 1 << 15; // TM = 1, LV = 0
            self.write(0x300, BCAST | INIT | TMLV); // set ICR 0x300
            const DS: u32 = 1 << 12;
            while self.read(0x300) & DS != 0 {} // wait for delivery status

            // FIXED: Enable interrupts on the APIC (but not on the processor).
            self.write(0x080, 0); // clear TPR to enable all interrupts
        }

        // NOTE: Try to use bitflags! macro to set the flags.
    }

    fn id(&self) -> u32 {
        // NOTE: Maybe you can handle regs like `0x0300` as a const.
        unsafe { self.read(0x0020) >> 24 }
    }

    fn version(&self) -> u32 {
        unsafe { self.read(0x0030) }
    }

    fn icr(&self) -> u64 {
        unsafe { (self.read(0x0310) as u64) << 32 | self.read(0x0300) as u64 }
    }

    fn set_icr(&mut self, value: u64) {
        unsafe {
            while self.read(0x0300).get_bit(12) {}
            self.write(0x0310, (value >> 32) as u32);
            self.write(0x0300, value as u32);
            while self.read(0x0300).get_bit(12) {}
        }
    }

    fn eoi(&mut self) {
        unsafe {
            self.write(0x00B0, 0);
        }
    }
}

impl Debug for XApic {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Xapic")
            .field("id", &self.id())
            .field("version", &self.version())
            .field("icr", &self.icr())
            .finish()
    }
}
