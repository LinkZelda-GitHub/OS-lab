use core::{
    fmt::{Debug, Error, Formatter},
    ptr::{read_volatile, write_volatile},
};

use bit_field::BitField;
use bitflags::bitflags;
use x86::{cpuid::CpuId};

use super::LocalApic;
use crate::memory::physical_to_virtual;
use crate::interrupt::consts::{Interrupts, Irq};

/// Default physical address of xAPIC
pub const LAPIC_ADDR: u64 = 0xFEE00000;

// 定义 APIC 寄存器偏移量常量
const REG_ID: u32 = 0x0020;
const REG_VER: u32 = 0x0030;
const REG_TPR: u32 = 0x0080;
const REG_EOI: u32 = 0x00B0;
const REG_SVR: u32 = 0x00F0;
const REG_ESR: u32 = 0x0280;
const REG_ICR_LOW: u32 = 0x0300;
const REG_ICR_HIGH: u32 = 0x0310;
const REG_LVT_TIMER: u32 = 0x0320;
const REG_LVT_PC: u32 = 0x0340;
const REG_LVT_LINT0: u32 = 0x0350;
const REG_LVT_LINT1: u32 = 0x0360;
const REG_LVT_ERROR: u32 = 0x0370;
const REG_TICR: u32 = 0x0380;
const REG_TDCR: u32 = 0x03E0;

bitflags! {
    /// LVT 寄存器的位定义 (Timer, LINT0, ...)
    struct LvtFlags: u32 {
        const MASKED = 1 << 16;
        const PERIODIC = 1 << 17;
        const TSC_DEADLINE = 1 << 18;
    }

    /// Spurious Interrupt Vector Register (SVR) 位定义
    struct SvrFlags: u32 {
        const ENABLE_APIC = 1 << 8;
        const FOCUS_PROCESSOR_CHECK = 1 << 9;
    }
}

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
            self.read(REG_ID); // 写入后读取以确保同步
        }
    }
}

impl LocalApic for XApic {
    fn support() -> bool {
        // FIXED: Check CPUID to see if xAPIC is supported.
        CpuId::new().get_feature_info().map(|f| f.has_apic()).unwrap_or(false)
    }

    fn cpu_init(&mut self) {
        unsafe {
            // FIXED: Enable local APIC; set spurious interrupt vector.
            let mut svr = self.read(REG_SVR);
            let mut flags = SvrFlags::from_bits_truncate(svr);
            flags.insert(SvrFlags::ENABLE_APIC);
            svr = flags.bits();
            svr.set_bits(0..8, (Interrupts::IrqBase as u32 + Irq::Spurious as u32));
            self.write(REG_SVR, svr);

            // FIXED: The timer repeatedly counts down at bus frequency
            let vector = Interrupts::IrqBase as u32 + Irq::Timer as u32;
            // 直接构建 flags，不包含 MASKED 即代表清除 Mask 位
            let mut timer_flags = LvtFlags::empty();
            timer_flags.insert(LvtFlags::PERIODIC); // set Timer Periodic Mode
            
            // 写入向量号与标志位的组合值，确保第16位为0
            self.write(REG_LVT_TIMER, vector | timer_flags.bits());

            self.write(REG_TDCR, 0b1011); // set Timer Divide to 1
            self.write(REG_TICR, 0x20000); // set initial count to 0x20000

            // FIXED: Disable logical interrupt lines (LINT0, LINT1)
            self.write(REG_LVT_LINT0, LvtFlags::MASKED.bits()); // set Mask LINT0
            self.write(REG_LVT_LINT1, LvtFlags::MASKED.bits()); // set Mask LINT1

            // FIXED: Disable performance counter overflow interrupts (PCINT)
            self.write(REG_LVT_PC, LvtFlags::MASKED.bits()); // set Mask PCINT

            // FIXED: Map error interrupt to IRQ_ERROR.
            self.write(REG_LVT_ERROR, Interrupts::IrqBase as u32 + Irq::Error as u32);

            // FIXED: Clear error status register (requires back-to-back
            // writes).
            self.write(REG_ESR, 0); // clear error status register
            self.write(REG_ESR, 0); // clear error status register Again

            // FIXED: Ack any outstanding interrupts.   
            self.eoi(); // write EOI as zero

            // FIXED: Send an Init Level De-Assert to synchronise arbitration
            // ID's.
            self.write(REG_ICR_HIGH, 0); // set ICR 0x310
            
            // 使用位域构建 ICR 低 32 位
            let mut icr_low: u32 = 0;
            icr_low.set_bits(18..20, 0b10); // BCAST (All including self) -> 1 << 19
            icr_low.set_bits(8..11, 0b101); // INIT -> 5 << 8
            icr_low.set_bit(15, true);      // TMLV (Level Assert) -> 1 << 15
            
            self.write(REG_ICR_LOW, icr_low);
            
            while self.read(REG_ICR_LOW).get_bit(12) {} // wait for delivery status (DS bit)

            // FIXED: Enable interrupts on the APIC (but not on the processor).
            self.write(REG_TPR, 0); // clear TPR to enable all interrupts
        }

        // NOTE: Try to use bitflags! macro to set the flags.
    }

    fn id(&self) -> u32 {
        // NOTE: Maybe you can handle regs like `0x0300` as a const.
        unsafe { self.read(REG_ID) >> 24 }
    }

    fn version(&self) -> u32 {
        unsafe { self.read(REG_VER) }
    }

    fn icr(&self) -> u64 {
        unsafe { (self.read(REG_ICR_HIGH) as u64) << 32 | self.read(REG_ICR_LOW) as u64 }
    }

    fn set_icr(&mut self, value: u64) {
        unsafe {
            while self.read(REG_ICR_LOW).get_bit(12) {}
            self.write(REG_ICR_HIGH, (value >> 32) as u32);
            self.write(REG_ICR_LOW, value as u32);
            while self.read(REG_ICR_LOW).get_bit(12) {}
        }
    }

    fn eoi(&mut self) {
        unsafe {
            self.write(REG_EOI, 0);
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

// xapic.rs(origin)
// use core::{
//     fmt::{Debug, Error, Formatter},
//     ptr::{read_volatile, write_volatile},
// };

// use bit_field::BitField;
// use x86::{cpuid::CpuId, segmentation::BuildDescriptor};

// use super::LocalApic;

// use crate::memory::physical_to_virtual;
// use crate::interrupt::consts::{Interrupts, Irq};

// /// Default physical address of xAPIC
// pub const LAPIC_ADDR: u64 = 0xFEE00000;

// pub struct XApic {
//     addr: u64,
// }

// impl XApic {
//     pub unsafe fn new(addr: u64) -> Self {
//         XApic { addr }
//     }

//     unsafe fn read(&self, reg: u32) -> u32 {
//         unsafe { read_volatile((self.addr + reg as u64) as *const u32) }
//     }

//     unsafe fn write(&mut self, reg: u32, value: u32) {
//         unsafe {
//             write_volatile((self.addr + reg as u64) as *mut u32, value);
//             self.read(0x20);
//         }
//     }
// }

// impl LocalApic for XApic {
//     /// If this type APIC is supported
//     fn support() -> bool {
//         // FIXED: Check CPUID to see if xAPIC is supported.
//         CpuId::new().get_feature_info().map(
//             |f| f.has_apic()
//         ).unwrap_or(false)
//     }

//     /// Initialize the xAPIC for the current CPU.
//     fn cpu_init(&mut self) {
//         unsafe {
//             // FIXED: Enable local APIC; set spurious interrupt vector.
//             let mut spiv = self.read(0xF0);
//             spiv |= 1<<8; // set EN bit
//             // clear and set Vector
//             spiv &= !(0xFF);
//             spiv |= Interrupts::IrqBase as u32 + Irq::Spurious as u32;
//             self.write(0xF0, spiv);

//             // FIXED: The timer repeatedly counts down at bus frequency

//             let mut lvt_timer = self.read(0x320);
//             // clear and set Vector
//             lvt_timer &= !(0xFF);
//             lvt_timer |= Interrupts::IrqBase as u32 + Irq::Timer as u32;
//             lvt_timer &= !(1 << 16); // clear Mask
//             lvt_timer |= 1 << 17; // set Timer Periodic Mode
//             self.write(0x320, lvt_timer);

//             self.write(0x3E0, 0b1011); // set Timer Divide to 1
//             // self.write(0x3E0, 0b0000); // Test: set Timer Divide to 2
//             // self.write(0x3E0, 0b1000); // Test set Timer Divide to 4
//             self.write(0x380, 0x20000); // set initial count to 0x20000

//             // FIXED: Disable logical interrupt lines (LINT0, LINT1)
//             self.write(0x350, 1 << 16); // set Mask LINT0
//             self.write(0x360, 1 << 16); // set Mask LINT1

//             // FIXED: Disable performance counter overflow interrupts (PCINT)
//             self.write(0x340, 1 << 16); // set Mask PCINT

//             // FIXED: Map error interrupt to IRQ_ERROR.
//             self.write(0x370, Interrupts::IrqBase as u32 + Irq::Error as u32);

//             // FIXED: Clear error status register (requires back-to-back
//             // writes).
//             self.write(0x280, 0); // clear error status register
//             self.write(0x280, 0); // clear error status register Again

//             // FIXED: Ack any outstanding interrupts.   
//             self.write(0x0b0, 0); // write EOI as zero

//             // FIXED: Send an Init Level De-Assert to synchronise arbitration
//             // ID's.
//             self.write(0x310, 0); // set ICR 0x310
//             const BCAST: u32 = 1 << 19;
//             const INIT: u32 = 5 << 8;
//             const TMLV: u32 = 1 << 15; // TM = 1, LV = 0
//             self.write(0x300, BCAST | INIT | TMLV); // set ICR 0x300
//             const DS: u32 = 1 << 12;
//             while self.read(0x300) & DS != 0 {} // wait for delivery status

//             // FIXED: Enable interrupts on the APIC (but not on the processor).
//             self.write(0x080, 0); // clear TPR to enable all interrupts
//         }

//         // NOTE: Try to use bitflags! macro to set the flags.
//     }

//     fn id(&self) -> u32 {
//         // NOTE: Maybe you can handle regs like `0x0300` as a const.
//         unsafe { self.read(0x0020) >> 24 }
//     }

//     fn version(&self) -> u32 {
//         unsafe { self.read(0x0030) }
//     }

//     fn icr(&self) -> u64 {
//         unsafe { (self.read(0x0310) as u64) << 32 | self.read(0x0300) as u64 }
//     }

//     fn set_icr(&mut self, value: u64) {
//         unsafe {
//             while self.read(0x0300).get_bit(12) {}
//             self.write(0x0310, (value >> 32) as u32);
//             self.write(0x0300, value as u32);
//             while self.read(0x0300).get_bit(12) {}
//         }
//     }

//     fn eoi(&mut self) {
//         unsafe {
//             self.write(0x00B0, 0);
//         }
//     }
// }

// impl Debug for XApic {
//     fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
//         f.debug_struct("Xapic")
//             .field("id", &self.id())
//             .field("version", &self.version())
//             .field("icr", &self.icr())
//             .finish()
//     }
// }