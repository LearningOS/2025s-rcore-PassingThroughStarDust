//! banker algorithm for deadlock avoidance
//  ** for chapter 8 exercises

use core::cmp::max;
use alloc::{vec, vec::Vec};

/// manages banker's algorithm safety check
/*
    I tried to wrap the inner structure with UPSafeCell,
    but it led to a "already borrowed: BorrowMutError" during execution.

    It should be because in multi-threaded scenarios, multiple threads might try to access
    the inner structure simultaneously, causing conflicts.

    Therefore, I decided to keep the inner structure as plain data,
    and require &mut self for methods that modify the state.
*/
pub struct Banker {
    /// available matrix
    available: Vec<isize>,
    /// allocation matrix
    allocation: Vec<Vec<isize>>,
    /// need matrix
    need: Vec<Vec<isize>>,
}

impl Banker {
    /// create new Banker
    pub fn new() -> Self {
        Banker {
            available: Vec::new(),
            allocation: Vec::new(),
            need: Vec::new()
        }
    }

    /// add new resource(allocation matrix and need matrix will be lazy expanded)
    pub fn add_new_resource_type(&mut self, rid: usize, count: isize) {
        let available = &mut self.available;
        while available.len() <= rid {
            available.push(0);
        }
        available[rid] = count;
    }

    /// expand sizes of allocation matrix and need matrix when fetching thread id or resource id
    /// on areas outside of current matrixies, assuming tid and rid has been checked as legal
    fn expand_matrixies(&mut self, tid: usize, rid: usize) {
        let allocation = &mut self.allocation;
        let need = &mut self.need;
        let available = &mut self.available;

        let resource_types = max(available.len(), rid);

        while allocation.len() <= tid {
            allocation.push(vec![0; resource_types]);
            need.push(vec![0; resource_types]);
        }

        for i in 0..allocation.len() {
            while allocation[i].len() < resource_types {
                allocation[i].push(0);
                need[i].push(0);
            }
        }
    }

    /// add new resource count to available matrix
    pub fn add_to_available(&mut self, rid: usize, count: isize) {
        let available = &mut self.available;
        available[rid] += count;
    }

    /// add new resource count to allocation matrix
    pub fn add_to_allocation(&mut self, tid: usize, rid: usize, count: isize) {
        self.expand_matrixies(tid, rid);
        let allocation = &mut self.allocation;
        // let allocation = &mut self.inner.allocation;
        allocation[tid][rid] += count;

    }

    /// add new resource count to allocation matrix
    pub fn add_to_need(&mut self, tid: usize, rid: usize, count: isize) {
        self.expand_matrixies(tid, rid);
        let need = &mut self.need;
        // let need = &mut self.inner.need;
        need[tid][rid] += count;

    }

    /// do safety check, return true if the current state is safe
    pub fn check_safety(&self) -> bool {
        let allocation = &self.allocation;
        let need = &self.need;
        let num_of_threads = allocation.len();
        let resource_types = self.available.len();
        let mut work = self.available.clone();
        let mut finish = vec![false; num_of_threads];
        let mut count = 0;  // number of finished threads


        // quit loop when no available thread to allocate or all threads finished
        let mut quit;
        loop {
            quit = true;
            for i in 0..num_of_threads {
                // thread i is not finished
                if !finish[i] {
                    // check if need[i,j] is less than or equal to work[j]
                    if (0..resource_types).all(|j| need[i][j] <= work[j]) {
                        for j in 0..resource_types {
                            // release all resources of this thread
                            work[j] += allocation[i][j];
                        }
                        finish[i] = true;
                        quit = false; // found a thread can be allocated
                        count += 1;
                    }
                }
            }

            if quit {
                break;
            }
        }

        count == num_of_threads
    }
}
