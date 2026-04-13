use core::ptr::addr_of_mut;

use lazy_static::lazy_static;
use x86_64::{
    VirtAddr,
    registers::segmentation::Segment,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const CLOCK_IST_INDEX: u16 = 2;
pub const SYSCALL_IST_INDEX: u16 = 3;

pub const IST_SIZES: [usize; 4] = [0x1000, 0x1000, 0x1000, 0x1000];

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // initialize the TSS with the static buffers
        // will be allocated on the bss section when the kernel is load
        //
        // DO NOT MODIFY THE FOLLOWING CODE
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = IST_SIZES[0];
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(addr_of_mut!(STACK));
            let stack_end = stack_start + STACK_SIZE as u64;
            info!(
                "Privilege Stack  : 0x{:016x}-0x{:016x}",
                stack_start.as_u64(),
                stack_end.as_u64()
            );
            stack_end
        };

        // FIXED: fill tss.interrupt_stack_table with the static stack buffers like above
        // You can use `tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize]`
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = IST_SIZES[DOUBLE_FAULT_IST_INDEX as usize];
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(addr_of_mut!(STACK));
            let stack_end = stack_start + STACK_SIZE as u64;
            info!(
                "Double Fault IST : 0x{:016x}-0x{:016x}",
                stack_start.as_u64(),
                stack_end.as_u64()
            );
            stack_end
        };

        tss.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = IST_SIZES[PAGE_FAULT_IST_INDEX as usize];
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(addr_of_mut!(STACK));
            let stack_end = stack_start + STACK_SIZE as u64;
            info!(
                "Page Fault IST   : 0x{:016x}-0x{:016x}",
                stack_start.as_u64(),
                stack_end.as_u64()
            );
            stack_end
        };

        tss.interrupt_stack_table[CLOCK_IST_INDEX as usize] = {
            const STACK_SIZE: usize = IST_SIZES[CLOCK_IST_INDEX as usize];
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(addr_of_mut!(STACK));
            let stack_end = stack_start + STACK_SIZE as u64;
            info!(
                "Clock IST        : 0x{:016x}-0x{:016x}",
                stack_start.as_u64(),
                stack_end.as_u64()
            );
            stack_end
        };

        tss.interrupt_stack_table[SYSCALL_IST_INDEX as usize] = {
            const STACK_SIZE: usize = IST_SIZES[SYSCALL_IST_INDEX as usize];
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(addr_of_mut!(STACK));
            let stack_end = stack_start + STACK_SIZE as u64;
            info!(
                "Syscall IST       : 0x{:016x}-0x{:016x}",
                stack_start.as_u64(),
                stack_end.as_u64()
            );
            stack_end
        };

        tss
    };
}

// lazy_static! {
//     static ref GDT: (GlobalDescriptorTable, KernelSelectors) = {
//         let mut gdt = GlobalDescriptorTable::new();
//         let code_selector = gdt.append(Descriptor::kernel_code_segment());
//         let data_selector = gdt.append(Descriptor::kernel_data_segment());
//         let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
//         (
//             gdt,
//             KernelSelectors {
//                 code_selector,
//                 data_selector,
//                 tss_selector,
//             },
//         )
//     };
// }

// #[derive(Debug)]
// pub struct KernelSelectors {
//     pub code_selector: SegmentSelector,
//     pub data_selector: SegmentSelector,
//     tss_selector: SegmentSelector,
// }

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        (
            gdt,
            Selectors {
                kernel_code_selector,
                kernel_data_selector,
                tss_selector,
                user_code_selector,
                user_data_selector,
            },
        )
    };
}

#[derive(Debug)]
pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
}

pub fn get_user_selector() -> &'static Selectors {
    &GDT.1
}

pub fn init() {
    use x86_64::{
        PrivilegeLevel,
        instructions::{
            segmentation::{CS, DS, ES, FS, GS, SS},
            tables::load_tss,
        },
    };

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.kernel_code_selector);
        DS::set_reg(GDT.1.kernel_data_selector);
        SS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        ES::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        FS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        GS::set_reg(SegmentSelector::new(0, PrivilegeLevel::Ring0));
        load_tss(GDT.1.tss_selector);
    }

    let mut size = 0;

    for &s in IST_SIZES.iter() {
        size += s;
    }

    let (size, unit) = crate::humanized_size(size as u64);
    info!("Kernel IST Size  : {:>7.*} {}", 3, size, unit);

    info!("GDT Initialized.");
}

pub fn get_selector() -> &'static Selectors {
    &GDT.1
}
