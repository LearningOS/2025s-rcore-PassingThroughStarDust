use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    //  ** for chapter 6 exercises
    /// Decrese the size of a disk inode
    fn decrease_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>
    ) {
        if new_size > disk_inode.size {
            return;
        }
        let old_block_num = disk_inode.data_blocks();
        let new_block_num = DiskInode::total_blocks(new_size);
        for inner_id in new_block_num..old_block_num {
            let block_id = disk_inode.get_block_id(inner_id, &self.block_device);
            fs.dealloc_data(block_id);
        }
        disk_inode.decrease_size(new_size, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
    //  ** for chapter 6 exercises
    /// Check if the size of the mapped disk inode reaches the maximum
    //  only for inode of root diretory. Since root inode allocates fixed size of bytes(diretory entry),
    //  so checking wheather the size is max could infer that whether the inode is still allocatable
    pub fn is_full(&self) -> bool {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.is_size_max()
        })
    }
    //  ** for chapter 6 exercises
    /// judge if this inode represents a diretory
    pub fn is_dir(&self) -> bool {
        let mut _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.is_dir()
        })
    }
    //  ** for chapter 6 exercises
    /// judge if this inode represents a file
    pub fn is_file(&self) -> bool {
        let mut _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.is_file()
        })
    }
    //  ** for chapter 6 exercises
    /// calculate the inode id (universal for all inodes)
    //  find_inode_id is only available for root inode
    pub fn get_inode_id(&self) -> u32 {
        let fs = self.fs.lock();
        fs.get_inode_id(self.block_id as u32, self.block_offset)
    }

    //  ** for chapter 6 exercises
    /// Hard link 2 paths (only used by inode of root diretory)
    pub fn linkat(&self, old_path: &str, new_path: &str) -> isize {
        assert!(!self.is_full());    // free space is needed for new diretory entry
        
        // check if the new path is already existed
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(new_path, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return -1;
        }

        // if the old path exists, allocate a new diretory entry for the new path
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|root_inode| {
            // get inode id of old path
            let Some(old_inode_id) = self.find_inode_id(old_path, root_inode) else { return -1; };

            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(new_path, old_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
            0
        })
    }

    //  ** for chapter 6 exercises
    /// Unlink a path to a file (only used by inode of root diretory)
    pub fn unlinkat(&self, path: &str) -> isize {
        /*
            // verify if path exists, and get reference to inode if exists
            let op = |root_inode: &DiskInode| {
                // assert it is a directory
                assert!(root_inode.is_dir());
                // has the file been created?
                self.find(path)
            };
            let Some(inode) = self.read_disk_inode(op) else { return -1; }; // DOUBLE read_disk_inode !!!!!!!
        */
        assert!(self.is_dir()); // very funny that if we deleted wrong inodes in one user app, then another app could failed in this assertation
        let Some(inode) = self.find(path) else { return -1; };
        let inode_id = inode.get_inode_id();
        // get number of links
        let nlink = {
            let mut sum: u32 = 0;
            for name in self.ls().iter() {
                if self.find(name).unwrap().get_inode_id() == inode_id {
                    sum += 1;
                }
            }
            sum
        };

        // if nlink == 1, delete inode and release related resources
        let mut fs = self.fs.lock();
        if nlink == 1 {
            // clear disk disk block size to zero and release related blocks
            // including disk_inode itself and indirect blocks)
            inode.modify_disk_inode(|disk_inode| {
                disk_inode.clear_size(&inode.block_device).into_iter().for_each(|block_id| {
                    // if using fs::dealloc_data, massive I/O (clearing bit on data blocks to 0) operation could lower the execution
                    // thus only dealloc the bits on data bitmap instead
                    fs.dealloc_data_weak(block_id);
                });
            });
            // cannot use inode's get_inode_id, because it acquires another lock of fs
            let inode_id = fs.get_inode_id(inode.block_id as u32, inode.block_offset) as usize;
            // erase disk inode itself(simply dealloc the bit in inode bitmap)
            fs.dealloc_inode(inode_id);
        }
        drop(inode);
        drop(fs);   // better keep this good habit to drop lock after at the end of a code block

        // erase corresponding diretory entry in the root diretory inode
        self.modify_disk_inode(|root_inode| {
            /*
                Since unlinkat can randomly delete a diretory entry from the diretory entry sequence in root
                inode, it is possible that the deleted diretory entry is in the middle thus creating a hollow.
                Here is a simple solution for this: read all the diretory entries into a vector, remove the
                target and rewrite the rest back to disk inode's blocks. There could have a more efficient
                implementation but it could brings considerable change to the code structure, thus the simple
                solution is adorable if just wanting to finish this lab.
            */
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let mut position = -1isize; // stores where the dirent to be deleted locates in the old dirent sequence
            let mut dirents = Vec::new();
            // scan the dirent sequence, stores dirent after the target dirent, leaving the preceeding dirents unchanged
            for i in 0..file_count {
                let offset = DIRENT_SZ * i;
                let mut dirent = DirEntry::empty();
                // read size must equal to dirent size
                assert!(root_inode.read_at(offset, dirent.as_bytes_mut(), &self.block_device) == DIRENT_SZ);
                // this if nesting structure reduces unuseful judgement for dirent name
                if position < 0 {
                    if dirent.name() == path { position = offset as isize; }
                    // the dirent to be deleted will be automatically ignored, and will be covered at the writing back stage
                } else {
                    dirents.push(dirent);
                }
            }
            let new_size = (file_count - 1) * DIRENT_SZ;
            root_inode.size = new_size as u32;
            // decrease size
            let mut fs = self.fs.lock();
            self.decrease_size(new_size as u32, root_inode, &mut fs);
            drop(fs);
            // write back temporarily stored dirents
            for (i, ref dirent) in dirents.into_iter().enumerate() {
                root_inode.write_at(
                    position as usize + i * DIRENT_SZ,
                    dirent.as_bytes(),
                    &self.block_device
                );      
            }
            0
        })
    }
}
