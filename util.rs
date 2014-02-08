//
// sprocketnes/util.rs
//
// Author: Patrick Walton
//

use std::io::File;
use std::libc::{c_int, c_void, time_t};
use std::ptr::null;

//
// Random number generation
//

pub struct Xorshift {
    x: u32,
    y: u32,
    z: u32,
    w: u32,
}

impl Xorshift {
    pub fn new() -> Xorshift {
        Xorshift { x: 123456789, y: 362436069, z: 521288629, w: 88675123 }
    }

    pub fn next(&mut self) -> u32 {
        let t = self.x ^ (self.x << 11);
        self.x = self.y; self.y = self.z; self.z = self.w;
        self.w = self.w ^ (self.w >> 19) ^ (t ^ (t >> 8));
        self.w
    }
}

//
// Simple assertions
//

#[cfg(debug)]
pub fn debug_assert(cond: bool, msg: &str) {
    if !cond {
        println(msg);
    }
}

#[cfg(not(debug))]
pub fn debug_assert(_: bool, _: &str) {}

#[cfg(debug)]
pub fn debug_print(msg: &str) {
    println(msg);
}

#[cfg(not(debug))]
pub fn debug_print(_: &str) {}

//
// Bindings for `gettimeofday(2)`
//

struct timeval {
    tv_sec: time_t,
    tv_usec: u32,
}

extern {
    fn gettimeofday(tp: *mut timeval, tzp: *c_void) -> c_int;
}

pub fn current_time_millis() -> u64 {
    unsafe {
        let mut tv = timeval { tv_sec: 0, tv_usec: 0 };
        gettimeofday(&mut tv, null());
        (tv.tv_sec as u64) * 1000 + (tv.tv_usec as u64) / 1000
    }
}

