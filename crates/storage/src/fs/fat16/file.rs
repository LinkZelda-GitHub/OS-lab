//! File
//!
//! reference: <https://wiki.osdev.org/FAT#Directories_on_FAT12.2F16.2F32>

use core::cmp::min;

use super::*;

#[derive(Debug, Clone)]
pub struct File {
    /// The current offset in the file
    offset: usize,
    /// The current cluster of this file
    current_cluster: Cluster,
    /// DirEntry of this file
    entry: DirEntry,
    /// The file system handle that contains this file
    handle: Fat16Handle,
}

impl File {
    pub fn new(handle: Fat16Handle, entry: DirEntry) -> Self {
        Self {
            offset: 0,
            current_cluster: entry.cluster,
            entry,
            handle,
        }
    }

    pub fn length(&self) -> usize {
        self.entry.size as usize
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> FsResult<usize> {
        // FIXED: read file content from disk
        //      CAUTION: file length / buffer size / offset
        //
        //      - `self.offset` is the current offset in the file in bytes
        //      - use `self.handle` to read the blocks
        //      - use `self.entry` to get the file's cluster
        //      - use `self.handle.cluster_to_sector` to convert cluster to sector
        //      - update `self.offset` after reading
        //      - update `self.cluster` with FAT if necessary
        if self.offset >= self.length() {
            return Ok(0); // EOF
        }

        let bytes_per_sector = self.handle.bpb.bytes_per_sector() as usize;
        let sectors_per_cluster = self.handle.bpb.sectors_per_cluster() as usize;
        let bytes_per_cluster = bytes_per_sector * sectors_per_cluster;

        // The remaining unread data length in file and the given buffer size
        let mut read_len = min(buf.len(), self.length() - self.offset);
        let total_to_read = read_len;
        let mut buf_offset = 0;

        while read_len > 0 {
            if self.current_cluster == Cluster::END_OF_FILE || self.current_cluster == Cluster::BAD
            {
                break;
            }

            let cluster_offset = self.offset % bytes_per_cluster;
            let sector_in_cluster = cluster_offset / bytes_per_sector;
            let offset_in_sector = cluster_offset % bytes_per_sector;

            let start_sector =
                self.handle.cluster_to_sector(&self.current_cluster) + sector_in_cluster;

            let mut block = Block::default();
            self.handle.inner.read_block(start_sector, &mut block)?;

            let available_in_sector = bytes_per_sector - offset_in_sector;
            let to_copy = min(read_len, available_in_sector);

            buf[buf_offset..buf_offset + to_copy]
                .copy_from_slice(&block.as_ref()[offset_in_sector..offset_in_sector + to_copy]);

            self.offset += to_copy;
            buf_offset += to_copy;
            read_len -= to_copy;

            // Update cluster if we crossed a cluster boundary
            if self.offset % bytes_per_cluster == 0 && self.offset < self.length() {
                self.current_cluster = self.handle.next_cluster(&self.current_cluster)?;
            }
        }

        Ok(total_to_read - read_len)
    }
}

// NOTE: `Seek` trait is not required for this lab
impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> FsResult<usize> {
        unimplemented!()
    }
}

// NOTE: `Write` trait is not required for this lab
impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> FsResult<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> FsResult {
        unimplemented!()
    }
}
