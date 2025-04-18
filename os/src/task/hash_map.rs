//! a simple hashmap used to count syscall
//!
//!
//!
use crate::syscall;

/// HashMap
#[derive(Clone, Copy)]
pub struct HashMap {
    inner: [(usize, isize); 5],
}

impl HashMap {
    /// new
    pub fn new() -> Self {
        let inner = [
            (syscall::SYSCALL_WRITE, 0isize),
            (syscall::SYSCALL_EXIT, 0),
            (syscall::SYSCALL_YIELD, 0),
            (syscall::SYSCALL_GET_TIME, 0),
            (syscall::SYSCALL_TRACE, 0),
        ];

        HashMap { inner }
    }

    /// incr
    pub fn incr(&mut self, syscall_type: usize) {
        for (t, v) in self.inner.iter_mut() {
            if syscall_type == *t {
                *v += 1;
            }
        }
    }

    /// get
    pub fn get(& self, syscall_type: usize) -> Option<isize> {
        for (t, v) in self.inner.iter() {
            if syscall_type == *t {
                return Some(*v);
            }
        }
        None
    }
}
