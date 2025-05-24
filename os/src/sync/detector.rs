//! ** for chapter 8 exercises
use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

use crate::task::current_task;

use super::UPSafeCell;

/// Thread Mutex Dead Lock Detector
pub struct Detector {
    /// source table
    pub inner: UPSafeCell<DetectorInner>,
}

impl Detector {
    /// create new MutexDetector
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(DetectorInner {
                    available: BTreeMap::new(),
                    allocation: BTreeMap::new(),
                    need: BTreeMap::new(),
                })
            },
        }
    }
}

/// Changeable Part of Thread Mutex Dead Lock Detector
pub struct DetectorInner {
    /// add when create, a lock represents a resource, its value is count, Mutex resource is set default as 1, and signal valueis decided it.
    pub available: BTreeMap<usize, u8>,
    /// resource owned by thread during waking up to unlock
    pub allocation: BTreeMap<usize, BTreeMap<usize, u8>>,
    /// thread waiting during lock
    pub need: BTreeMap<usize, BTreeMap<usize, u8>>,
}

impl DetectorInner {
    /// initialize available lock
    pub fn set_available(&mut self, id: usize, count: usize) {
        self.available.insert(id, count as u8);
    }
    
    /// add avaliable lock
    pub fn add_available(&mut self, id: usize) {
        *self.available.entry(id).or_insert(0) += 1;
    }
    
    /// set lock as unavaliable
    pub fn remove_available(&mut self, id: usize) {
        if let Some(count) = self.available.get_mut(&id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
    
    /// add allocation lock
    pub fn add_allocation(&mut self, id: usize) {
        let task = current_task().unwrap();
        let thread_id = task.inner_exclusive_access().res.as_ref().unwrap().tid;
        
        let thread_alloc = self.allocation.entry(thread_id).or_insert_with(BTreeMap::new);
        *thread_alloc.entry(id).or_insert(0) += 1;
    }
    
    /// remove allocation lock
    pub fn remove_allocation(&mut self, id: usize) {
        let task = current_task().unwrap();
        let thread_id = task.inner_exclusive_access().res.as_ref().unwrap().tid;
        
        if let Some(thread_alloc) = self.allocation.get_mut(&thread_id) {
            if let Some(count) = thread_alloc.get_mut(&id) {
                if *count > 0 {
                    *count -= 1;
                }
                if *count == 0 {
                    thread_alloc.remove(&id);
                }
            }
        }
    }
    
    /// add need lock
    pub fn add_need(&mut self, id: usize) {
        let task = current_task().unwrap();
        let thread_id = task.inner_exclusive_access().res.as_ref().unwrap().tid;
        
        let thread_need = self.need.entry(thread_id).or_insert_with(BTreeMap::new);
        *thread_need.entry(id).or_insert(0) += 1;
    }
    
    /// remove need lock
    pub fn remove_need(&mut self, id: usize) {
        let task = current_task().unwrap();
        let thread_id = task.inner_exclusive_access().res.as_ref().unwrap().tid;
        
        if let Some(thread_need) = self.need.get_mut(&thread_id) {
            if let Some(count) = thread_need.get_mut(&id) {
                if *count > 0 {
                    *count -= 1;
                }
                if *count == 0 {
                    thread_need.remove(&id);
                }
            }
        }
    }

    /// detect deadlock
    pub fn detect_deadlock(&self) -> bool {
        // create a set of all thread id
        let mut all_thread_ids = Vec::new();
        for &thread_id in self.allocation.keys() {
            if !all_thread_ids.contains(&thread_id) {
                all_thread_ids.push(thread_id);
            }
        }
        for &thread_id in self.need.keys() {
            if !all_thread_ids.contains(&thread_id) {
                all_thread_ids.push(thread_id);
            }
        }
        
        // prepare work vector
        let mut work = self.available.clone();
        let mut finish = vec![false; all_thread_ids.len()];
        
        let mut found = true;
        while found {
            found = false;
            for (idx, &thread_id) in all_thread_ids.iter().enumerate() {
                if !finish[idx] {
                    // check if the demands of all threads on resource is fitted
                    let mut can_allocate = true;
                    
                    if let Some(thread_need) = self.need.get(&thread_id) {
                        for (&res_id, &need_count) in thread_need {
                            let available_count = *work.get(&res_id).unwrap_or(&0);
                            if need_count > available_count {
                                can_allocate = false;
                                break;
                            }
                        }
                    }
                    
                    if can_allocate {
                        // If all resources can be satisfied, it is marked as completed and the allocated resources are released.
                        if let Some(thread_alloc) = self.allocation.get(&thread_id) {
                            for (&res_id, &alloc_count) in thread_alloc {
                                *work.entry(res_id).or_insert(0) += alloc_count;
                            }
                        }
                        finish[idx] = true;
                        found = true; // Find at least one thread that can complete and continue the loop
                    }
                }
            }
        }
        
        // Check if all threads have finished
        finish.iter().any(|&x| !x) // If there are unfinished threads, there is a deadlock.
    }
}