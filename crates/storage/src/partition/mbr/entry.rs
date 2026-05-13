//! Partition Metadata
//!
//! This struct represents partitions' metadata.

use super::*;

#[derive(Clone, Copy, Default)]
pub struct MbrPartition {
    data: [u8; 16],
}

impl MbrPartition {
    /// Parse a partition entry from the given data.
    pub fn parse(data: &[u8; 16]) -> MbrPartition {
        MbrPartition {
            data: data.to_owned(),
        }
    }

    // FIXED: define other fields in the MbrPartition
    //      - use `define_field!` macro
    //      - ensure you can pass the tests
    //      - you may change the field names if you want
    //
    //  NOTE: some fields are not aligned with byte.
    //      define your functions to extract values:
    //
    //      0x02 - 0x03 begin sector & begin cylinder
    //      0x06 - 0x07 end sector & end cylinder

    // an example of how to define a field
    // move your mouse on the `define_field!` to see the docs
    define_field!(u8, 0x00, status); // Status (bit 7 set = active or bootable)
    define_field!([u8; 3], 0x01, chs_start); // CHS address of first partition sector (3 bytes)
    define_field!(u8, 0x04, partition_type); // Partition type
    define_field!([u8; 3], 0x05, chs_end); // CHS address of last partition sector (3 bytes)
    define_field!(u32, 0x08, begin_lba); // LBA of partition start
    define_field!(u32, 0x0C, total_lba); // Number of sectors in partition

    pub fn is_active(&self) -> bool {
        self.status() == 0x80
    }

    pub fn begin_head(&self) -> u8 {
        self.chs_start()[0]
    }

    pub fn begin_sector(&self) -> u8 {
        self.chs_start()[1] & 0x3F
    }

    pub fn begin_cylinder(&self) -> u16 {
        ((self.chs_start()[1] as u16 & 0xC0) << 2) | self.chs_start()[2] as u16
    }

    pub fn end_head(&self) -> u8 {
        self.chs_end()[0]
    }

    pub fn end_sector(&self) -> u8 {
        self.chs_end()[1] & 0x3F
    }

    pub fn end_cylinder(&self) -> u16 {
        ((self.chs_end()[1] as u16 & 0xC0) << 2) | self.chs_end()[2] as u16
    }
}

impl core::fmt::Debug for MbrPartition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Partition Meta Data")
            .field("Active", &self.is_active())
            .field("Begin Head", &format!("0x{:02x}", self.begin_head()))
            .field("Begin Sector", &format!("0x{:04x}", self.begin_sector()))
            .field(
                "Begin Cylinder",
                &format!("0x{:04x}", self.begin_cylinder()),
            )
            .field(
                "Partition Type",
                &format!("0x{:02x}", self.partition_type()),
            )
            .field("End Head", &format!("0x{:02x}", self.end_head()))
            .field("End Sector", &format!("0x{:04x}", self.end_sector()))
            .field("End Cylinder", &format!("0x{:04x}", self.end_cylinder()))
            .field("Begin LBA", &format!("0x{:08x}", self.begin_lba()))
            .field("Total LBA", &format!("0x{:08x}", self.total_lba()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_test() {
        let data = hex_literal::hex!("80 01 01 00 0b fe bf fc 3f 00 00 00 7e 86 bb 00");

        let meta = MbrPartition::parse(&data);

        println!("{:#?}", meta);

        assert!(meta.is_active());
        assert_eq!(meta.begin_head(), 1);
        assert_eq!(meta.begin_sector(), 1);
        assert_eq!(meta.begin_cylinder(), 0);
        assert_eq!(meta.partition_type(), 0x0b);
        assert_eq!(meta.end_head(), 254);
        assert_eq!(meta.end_sector(), 63);
        assert_eq!(meta.end_cylinder(), 764);
        assert_eq!(meta.begin_lba(), 63);
        assert_eq!(meta.total_lba(), 12289662);
    }
}
