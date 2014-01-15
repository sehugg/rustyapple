//
// sprocketnes/mem.rs
//
// Author: Patrick Walton
//

//
// The memory interface
//

/// The basic memory interface
pub trait Mem {
    fn loadb(&mut self, addr: u16) -> u8;
    fn storeb(&mut self, addr: u16, val: u8);
}

pub trait MemUtil {
    fn loadw(&mut self, addr: u16) -> u16;
    fn storew(&mut self, addr: u16, val: u16);
    fn loadw_zp(&mut self, addr: u8) -> u16;
}

impl<M:Mem> MemUtil for M {
    fn loadw(&mut self, addr: u16) -> u16 {
        self.loadb(addr) as u16 | (self.loadb(addr + 1) as u16 << 8)
    }
    fn storew(&mut self, addr: u16, val: u16) {
        self.storeb(addr, (val & 0xff) as u8);
        self.storeb(addr + 1, ((val >> 8) & 0xff) as u8);
    }
    // Like loadw, but has wraparound behavior on the zero page for address 0xff.
    fn loadw_zp(&mut self, addr: u8) -> u16 {
        self.loadb(addr as u16) as u16 | (self.loadb((addr + 1) as u16) as u16 << 8)
    }
}

