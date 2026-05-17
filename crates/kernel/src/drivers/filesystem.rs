use alloc::{boxed::Box, format, string::String};

use chrono::DateTime;
use storage::{fat16::Fat16, mbr::*, *};

use super::ata::*;

pub static ROOTFS: spin::Once<Mount> = spin::Once::new();

pub fn get_rootfs() -> &'static Mount {
    ROOTFS.get().unwrap()
}

pub fn init() {
    info!("Opening disk device...");

    let drive = AtaDrive::open(0, 0).expect("Failed to open disk device");

    // only get the first partition
    let part = MbrTable::parse(drive)
        .expect("Failed to parse MBR")
        .partitions()
        .expect("Failed to get partitions")
        .remove(0);

    info!("Mounting filesystem...");

    ROOTFS.call_once(|| Mount::new(Box::new(Fat16::new(part)), "/".into()));

    trace!("Root filesystem: {:#?}", ROOTFS.get().unwrap());

    info!("Initialized Filesystem.");
}

pub fn ls(root_path: &str) {
    let iter = match get_rootfs().read_dir(root_path) {
        Ok(iter) => iter,
        Err(err) => {
            warn!("{:?}", err);
            return;
        }
    };

    // FIXED: format and print the file metadata
    //      - use `for meta in iter` to iterate over the entries
    //      - use `crate::humanized_size_short` for file size
    //      - add '/' to the end of directory names
    //      - format the date as you like
    //      - do not forget to print the table header
    println!("{:<15} {:>10} {:<20}", "Name", "Size", "Created");
    println!("{:-<15} {:->10} {:-<20}", "", "", "");

    for meta in iter {
        if meta.name == "unknown" {
            continue;
        }

        let mut name = meta.name.clone();
        if meta.entry_type == storage::FileType::Directory {
            name.push('/');
        }

        let size_str = if meta.entry_type == storage::FileType::Directory {
            String::from("-")
        } else {
            let (size, unit) = crate::utils::humanized_size_short(meta.len as u64);
            format!("{:.1} {}", size, unit)
        };

        let created_str = meta
            .created
            .map(|t| format!("{}", t.format("%Y-%m-%d %H:%M:%S")))
            .unwrap_or_else(|| String::from("Unknown"));

        println!("{:<15} {:>10} {:<20}", name, size_str, created_str);
    }
}
