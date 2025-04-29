//! a simple hashmap used to count syscall
//!
//!
//!

/// HashMap
#[derive(Clone, Copy)]
pub struct HashMap {
    inner: [i32; 510],
}

impl HashMap {
    /// new
    pub fn new() -> Self {
        let inner  = [0; 510];
        HashMap { inner }
    }

    /// incr
    pub fn incr(&mut self, syscall_type: usize) {
        self.inner[syscall_type] += 1;
    }

    /// get
    pub fn get(&self, syscall_type: usize) -> Option<isize> {
        Some(self.inner[syscall_type] as isize)
    }
}
