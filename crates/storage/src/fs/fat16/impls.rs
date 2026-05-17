use super::*;
use crate::alloc::string::ToString;

impl Fat16Impl {
    pub fn new(inner: impl BlockDevice<Block512>) -> Self {
        let mut block = Block::default();
        let block_size = Block512::size();

        inner.read_block(0, &mut block).unwrap();
        let bpb = Fat16Bpb::new(block.as_ref()).unwrap();

        trace!("Loading Fat16 Volume: {:#?}", bpb);

        // HINT: FirstDataSector = BPB_ResvdSecCnt + (BPB_NumFATs * FATSz) +
        // RootDirSectors;
        let fat_start = bpb.reserved_sector_count() as usize;

        let root_entries_count = bpb.root_entries_count() as usize;
        let byte_per_sector = bpb.bytes_per_sector() as usize;
        let root_dir_size = {
            /* FIXED: get the size of root dir from bpb */
            (root_entries_count * 32 + byte_per_sector - 1) / byte_per_sector
        };
        let first_root_dir_sector = {
            /* FIXED: calculate the first root dir sector */
            fat_start + (bpb.fat_count() as usize * bpb.sectors_per_fat() as usize)
        };
        let first_data_sector = first_root_dir_sector + root_dir_size;

        Self {
            bpb,
            inner: Box::new(inner),
            fat_start,
            first_data_sector,
            first_root_dir_sector,
        }
    }

    pub fn cluster_to_sector(&self, cluster: &Cluster) -> usize {
        match *cluster {
            Cluster::ROOT_DIR => self.first_root_dir_sector,
            Cluster(c) => {
                // FIXED: calculate the first sector of the cluster
                // HINT: FirstSectorofCluster = ((N – 2) * BPB_SecPerClus) +
                // FirstDataSector;
                if c >= 2 && c <= 0xFFF6 {
                    ((c as usize - 2) * self.bpb.sectors_per_cluster() as usize)
                        + self.first_data_sector
                } else {
                    panic!("Invalid cluster number: {}", c);
                }
            }
        }
    }

    // FIXED: YOU NEED TO IMPLEMENT THE FILE SYSTEM OPERATIONS HERE
    //      - read the FAT and get next cluster
    //      - traverse the cluster chain and read the data
    //      - parse the path
    //      - open the root directory
    //      - ...
    //      - finally, implement the FileSystem trait for Fat16 with `self.handle`

    /// read the FAT and get next cluster
    pub fn next_cluster(&self, cluster: &Cluster) -> FsResult<Cluster> {
        let fat_offset = cluster.0 as usize * 2;
        let fat_sector = self.fat_start + (fat_offset as usize / BLOCK_SIZE);
        let fat_entry_offset = fat_offset as usize % BLOCK_SIZE;

        let mut block = Block::default();
        self.inner.read_block(fat_sector, &mut block)?;

        let next_cluster_value = u16::from_le_bytes(
            block.as_ref()[fat_entry_offset..fat_entry_offset + 2]
                .try_into()
                .unwrap(),
        ) as u32;

        if next_cluster_value >= 0xFFF8 {
            return Err(FsError::EndOfFile);
        } else if next_cluster_value == 0xFFF7 {
            return Err(FsError::BadCluster);
        }

        Ok(Cluster(next_cluster_value))
    }

    /// Get entries inside a specific directory cluster
    pub fn get_dir_entries(&self, dir: &Directory) -> FsResult<Vec<DirEntry>> {
        let mut entries = Vec::new();
        let mut current_cluster = dir.cluster;
        let is_root = current_cluster == Cluster::ROOT_DIR;

        loop {
            let start_sector = self.cluster_to_sector(&current_cluster);
            let sectors_to_read = if is_root {
                let root_entries = self.bpb.root_entries_count() as usize;
                let bytes_per_sector = self.bpb.bytes_per_sector() as usize;
                (root_entries * 32 + bytes_per_sector - 1) / bytes_per_sector
            } else {
                self.bpb.sectors_per_cluster() as usize
            };

            for sector_offset in 0..sectors_to_read {
                let mut block = Block::default();
                self.inner
                    .read_block(start_sector + sector_offset, &mut block)?;

                for i in 0..(512 / DirEntry::LEN) {
                    let offset = i * DirEntry::LEN;
                    let buf = &block.as_ref()[offset..offset + DirEntry::LEN];

                    if buf[0] == 0x00 {
                        // End of directory
                        return Ok(entries);
                    }
                    if buf[0] == 0xE5 {
                        // Deleted entry
                        continue;
                    }

                    if let Ok(entry) = DirEntry::parse(buf) {
                        entries.push(entry);
                    }
                }
            }

            if is_root {
                break;
            }

            current_cluster = self.next_cluster(&current_cluster)?;
            if current_cluster == Cluster::END_OF_FILE || current_cluster == Cluster::BAD {
                break;
            }
        }
        Ok(entries)
    }

    /// Traverse the directory tree to find an entry by path
    pub fn find_entry(&self, path: &str) -> FsResult<DirEntry> {
        let mut current_dir = Directory::root();

        // Remove trailing or leading slashes and split
        let path = path.trim_start_matches('/').trim_end_matches('/');
        if path.is_empty() {
            return Err(FsError::FileNotFound);
        }
        let parts: Vec<&str> = path.split('/').collect();

        let mut final_entry = None;

        for (i, part) in parts.iter().enumerate() {
            let entries = self.get_dir_entries(&current_dir)?;
            let parsed_name = ShortFileName::parse(part)?;

            let matched_entry = entries
                .into_iter()
                .find(|e| e.is_valid() && !e.is_long_name() && e.filename.matches(&parsed_name));

            if let Some(entry) = matched_entry {
                final_entry = Some(entry.clone());
                if i < parts.len() - 1 {
                    if entry.is_directory() {
                        let c = if entry.cluster.0 == 0 {
                            Cluster::ROOT_DIR
                        } else {
                            entry.cluster
                        };
                        current_dir = Directory::new(c);
                    } else {
                        return Err(FsError::NotADirectory);
                    }
                }
            } else {
                return Err(FsError::FileNotFound);
            }
        }

        final_entry.ok_or(FsError::FileNotFound)
    }
}

impl FileSystem for Fat16 {
    fn read_dir(&self, path: &str) -> FsResult<Box<dyn Iterator<Item = Metadata> + Send>> {
        // FIXED: read dir and return an iterator for all entries
        let dir = if path == "/" || path == "" {
            Directory::root()
        } else {
            let entry = self.handle.find_entry(path)?;
            if !entry.is_directory() {
                return Err(FsError::NotADirectory);
            }
            let c = if entry.cluster.0 == 0 {
                Cluster::ROOT_DIR
            } else {
                entry.cluster
            };
            Directory::new(c)
        };

        let entries = self.handle.get_dir_entries(&dir)?;
        let metadata: Vec<Metadata> = entries.iter().map(|e| e.as_meta()).collect();
        Ok(Box::new(metadata.into_iter()))
    }

    fn open_file(&self, path: &str) -> FsResult<FileHandle> {
        // FIXED: open file and return a file handle
        let entry = self.handle.find_entry(path)?;
        if entry.is_directory() {
            return Err(FsError::InvalidOperation);
        }

        let meta = entry.as_meta();
        let file = Box::new(File::new(self.handle.clone(), entry));

        Ok(FileHandle::new(meta, file))
    }

    fn metadata(&self, path: &str) -> FsResult<Metadata> {
        // FIXED: read metadata of the file / dir
        if path == "/" || path.is_empty() {
            // Root directory fake metadata
            return Ok(Metadata {
                entry_type: FileType::Directory,
                name: "/".to_string(),
                len: 0,
                created: None,
                accessed: None,
                modified: None,
            });
        }
        let entry = self.handle.find_entry(path)?;
        Ok(entry.as_meta())
    }

    fn exists(&self, path: &str) -> FsResult<bool> {
        // FIXED: check if the file / dir exists
        if path == "/" || path.is_empty() {
            return Ok(true);
        }
        Ok(self.handle.find_entry(path).is_ok())
    }
}
