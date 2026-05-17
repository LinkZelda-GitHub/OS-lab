use core::marker::PhantomData;

use crate::*;

pub mod mbr;

/// Partition table trait
pub trait PartitionTable<T, B>
where
    T: BlockDevice<B>,
    B: BlockTrait,
    Self: Sized,
{
    /// Parse the partition table
    fn parse(inner: T) -> FsResult<Self>;

    /// Returns the partitions
    fn partitions(&self) -> FsResult<Vec<Partition<T, B>>>;
}

/// Identifies a partition on the disk.
#[derive(Clone, Copy)]
pub struct Partition<T, B>
where
    T: BlockDevice<B>,
    B: BlockTrait,
{
    inner: T,
    offset: usize,
    size: usize,
    part_type: u8,
    _block: PhantomData<B>,
}

impl<T, B> Partition<T, B>
where
    T: BlockDevice<B>,
    B: BlockTrait,
{
    pub fn new(inner: T, offset: usize, size: usize, part_type: u8) -> Self {
        Self {
            inner,
            offset,
            size,
            part_type,
            _block: PhantomData,
        }
    }
}

impl<T, B> core::fmt::Debug for Partition<T, B>
where
    T: BlockDevice<B>,
    B: BlockTrait,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Partition")
            .field("type", &format!("0x{:02x}", self.part_type))
            .field("offset", &self.offset)
            .field("size", &self.size)
            .finish()
    }
}

impl<T, B> BlockDevice<B> for Partition<T, B>
where
    T: BlockDevice<B>,
    B: BlockTrait,
{
    fn block_count(&self) -> FsResult<usize> {
        self.inner.block_count()
    }

    fn read_block(&self, offset: usize, block: &mut B) -> FsResult {
        if offset >= self.size {
            return Err(FsError::InvalidOffset);
        }

        // FIXED: calculate the block offset for inner device
        let inner_offset = self.offset + offset;
        // FIXED: read from the inner device
        self.inner.read_block(inner_offset, block)
    }

    fn write_block(&self, offset: usize, block: &B) -> FsResult {
        if offset >= self.size {
            return Err(FsError::InvalidOffset);
        }

        // FIXED: calculate the block offset for inner device
        let inner_offset = self.offset + offset;
        // FIXED: write to the inner device
        self.inner.write_block(inner_offset, block)
    }
}
