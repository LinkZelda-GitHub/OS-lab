#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::{boxed::Box, vec};

use elf::{load_elf, map_physical_memory, map_range};
use uefi::{Status, entry, mem::memory_map::MemoryMap};
use x86_64::registers::control::*;
use xmas_elf::ElfFile;
use ysos_boot::{config::Config, *};

mod config;

const CONFIG_PATH: &str = "\\EFI\\BOOT\\boot.conf";

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");

    log::set_max_level(log::LevelFilter::Info);
    info!("Running UEFI bootloader...");

    // 1. Load config
    let config = {
        let mut file = open_file(CONFIG_PATH);
        let buf = load_file(&mut file);
        Config::parse(buf)
        // FIXED
    };

    info!("Config: {:#x?}", config);

    // 2. Load ELF files
    let elf = {
        let mut file = open_file(config.kernel_path);
        let buf = load_file(&mut file);
        ElfFile::new(buf).expect("Invalid ELF file")
        // FIXED
    };

    unsafe {
        set_entry(elf.header.pt2.entry_point() as usize);
    }

    // 3. Load MemoryMap
    let mmap = uefi::boot::memory_map(MemoryType::LOADER_DATA).expect("Failed to get memory map");

    let max_phys_addr = mmap
        .entries()
        .map(|m| m.phys_start + m.page_count * 0x1000)
        .max()
        .unwrap()
        .max(0x1_0000_0000); // include IOAPIC MMIO area

    // 4. Map ELF segments, kernel stack and physical memory to virtual memory
    let mut page_table = current_page_table();

    // FIXED: root page table is readonly, disable write protect (Cr0)
    unsafe {
        Cr0::update(|flags| {
            flags.remove(Cr0Flags::WRITE_PROTECT);
        });
    }

    // FIXED: map physical memory to specific virtual address offset
    let mut frame_allocator = UEFIFrameAllocator;

    map_physical_memory(
        config.physical_memory_offset,
        max_phys_addr,
        &mut page_table,
        &mut frame_allocator,
    );

    // FIXED: load and map the kernel elf file
    let elf_pages = load_elf(
        &elf,
        config.physical_memory_offset,
        &mut page_table, // load_elf expects Mapper<Size4KiB>
        &mut frame_allocator,
        false,
    )
    .expect("Failed to load ELF");

    let mut kernel_pages = arrayvec::ArrayVec::new();
    for range in &elf_pages {
        let _ = kernel_pages.try_push(*range);
    }

    // FIXED: map kernel stack
    map_range(
        config.kernel_stack_address,
        config.kernel_stack_size, // number of 4KiB pages
        &mut page_table,
        &mut frame_allocator,
    )
    .expect("Failed to map kernel stack");

    // FIXED: recover write protect (Cr0)
    unsafe {
        Cr0::update(|flags| {
            flags.insert(Cr0Flags::WRITE_PROTECT);
        });
    }

    free_elf(elf);

    // 5. Pass system table to kernel
    let ptr = uefi::table::system_table_raw().expect("Failed to get system table");
    let system_table = ptr.cast::<core::ffi::c_void>();

    // 6. Load apps
    let apps = if config.load_apps {
        info!("Loading apps...");
        Some(load_apps())
    } else {
        info!("Skip loading apps");
        None
    };

    // 7. Exit boot and jump to ELF entry
    info!("Exiting boot services...");

    let mmap = unsafe { uefi::boot::exit_boot_services(None) };
    // NOTE: alloc & log are no longer available

    // construct BootInfo
    let bootinfo = BootInfo {
        memory_map: mmap.entries().copied().collect(),
        physical_memory_offset: config.physical_memory_offset,
        system_table,
        log_lever: config.log_level,
        loaded_apps: apps,
        kernel_pages,
    };

    // align stack to 8 bytes
    let stacktop = config.kernel_stack_address + config.kernel_stack_size * 0x1000 - 8;

    jump_to_entry(&bootinfo, stacktop);
}
