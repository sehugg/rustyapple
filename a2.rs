
use mem::Mem;
use util::Xorshift;

static GR_TXMODE:  int = 1;
static GR_MIXMODE: int = 2;
static GR_PAGE1:   int = 4;
static GR_HIRES:   int = 8;

static HW_LO:	   u16 = 0xC000;
static ROM_LO: 	   u16 = 0xD000;
static ROM_LEN:	   u16 = 0x3000;

pub trait Peripheral
{
    fn doIO(&mut self, addr: u16, val: u8) -> u8;
    fn doHighIO(&mut self, addr: u16, val: u8) -> u8;
}

struct LangCardState
{
   // language card switches
   auxRAMselected: bool,
   auxRAMbank: u8,
   writeinhibit: bool,

   // value to add when reading & writing each of these banks
   // bank 1 is D000-FFFF, bank 2 is D000-DFFF
   bank1rdoffset: int,
   bank2rdoffset: int,
   bank1wroffset: int,
   bank2wroffset: int,
}

impl LangCardState
{
   fn new(auxRAMselected: bool, auxRAMbank: u8, writeinhibit: bool) -> LangCardState {
      LangCardState {
         auxRAMselected: auxRAMselected,
         auxRAMbank: auxRAMbank,
         writeinhibit: writeinhibit,
         // reset language card constants
         // 0x3000  = map 0xd000-0xffff -> 0x10000-0x12fff
         // -0x1000 = map 0xd000-0xdfff -> 0xc000-0xcfff
         bank1rdoffset: if auxRAMselected { 0x3000 } else { 0x0 },
         bank2rdoffset: if auxRAMselected { if auxRAMbank==2 { -0x1000 } else { 0x3000 } } else { 0x0 },
         bank1wroffset: if !writeinhibit { 0x3000 } else { 0x0 },
         bank2wroffset: if !writeinhibit { if auxRAMbank==2 { -0x1000 } else { 0x3000 } } else { 0x0 },
      }
   }
}

pub struct AppleII
{
   mem: [u8, ..0x13000],
   
   slots: [Option<~Peripheral>, ..8],
    
   debugflags: int,
   kbdlatch: u8,
   grswitch: u16,
   soundstate: bool,
   aux: LangCardState,
   nreads: u16 // counts # of reads for noise() fn
}

impl Mem for AppleII
{
    fn loadb(&mut self, addr: u16) -> u8
    {
       self.nreads += 1;
       let val =
      // see if it's from main memory (0x0000-0xbfff)
      if (addr < HW_LO) {
         self.mem[addr] & 0xff
      // see if it came from the ROM/LC area (0xd000-0xffff)
      } else if (addr >= ROM_LO) {
         if (addr >= 0xe000) {
            self.mem[addr as int + self.aux.bank1rdoffset] & 0xff
         } else {
            self.mem[addr as int + self.aux.bank2rdoffset] & 0xff
         }
      }
      // it must be an I/O location (0xc000-0xcfff)
      else if (addr < HW_LO + 0x100) {
         let noise = self.noise(); // when reading, pass noise as value (we might get it back)
         self.doIO(addr, noise)
      } else {
         match self.slots[(addr >> 8) & 7] {
            None    => self.noise(),
            Some(ref mut p) => p.doHighIO(addr, 0) // TODO: maybe have optional value, or new method
         }
      };
      debug!("Read {:x} = {:x}", addr, val);
      return val;
    }
    
    fn storeb(&mut self, addr: u16, val: u8)
    {
       debug!("Write {:x} = {:x}", addr, val);
      // see if it's from main memory (0x0000-0xbfff)
      if (addr < HW_LO)
      {
         self.mem[addr] = val;
         //dirty[addr >> 7] = true;
      }
      // see if it came from the ROM/LC area (0xd000-0xffff)
      else if (addr >= ROM_LO && /* auxRAMselected && */ !self.aux.writeinhibit)
      {
         if (addr >= 0xe000) {
            self.mem[addr as int + self.aux.bank1wroffset] = val;
         } else {
            self.mem[addr as int + self.aux.bank2wroffset] = val;
         }
      }
      // it must be an I/O location (0xc000-0xcfff)
      else if (addr < HW_LO + 0x100) {
         self.doIO(addr, val);
      } else {
         match self.slots[(addr >> 8) & 7] {
            None    => (), // no-op
            Some(ref mut p) => { p.doHighIO(addr, val); }
         }
      }
    }
}

impl AppleII
{
    pub fn new() -> AppleII { AppleII { 
       mem:   [ 0, ..0x13000 ],
       // TODO: slots: [ None, ..8 ],
       // https://gist.github.com/carl-eastlund/6264938
       slots: [ None, None, None, None, None, None, None, None ],
       aux:   LangCardState::new(false, 1, true),
       debugflags: 0,
       kbdlatch: 0,
       grswitch: 0,
       soundstate: false,
       nreads: 0
    } }
    
    pub fn set_slot(&mut self, slot: uint, mut p: ~Peripheral)
    {
      //p.doIO(0,0);
      self.slots[slot] = Some(p);
      //self.slots[slot].get_mut_ref().doIO(0,0);
    }
    
    fn noise(&mut self) -> u8 { self.mem[self.nreads & 0xffff] }
    
    fn setGrSwitch(&mut self, addr: u16)
    {
      // graphics
      if ((addr & 1) != 0) {
         self.grswitch |= 1 << ((addr >> 1) & 0x07);
      } else {
         self.grswitch &= !(1 << ((addr >> 1) & 0x07));
      }
      debug!("switch {} grswitch = {}", addr, self.grswitch);
    }
    
    fn setAnnunciator(&mut self, addr: u16)
    {
       // nothing yet
    }
    
    fn fakeJoystick(&mut self, addr: u16) -> u8
    {
         // tapein, joystick, buttons
         match addr & 7 {
            1..3 => self.noise() & 0x7f,	// buttons (off)
            4..5 => self.noise() | 0x80,	// joystick
            _    => self.noise()
         }
    }
    
    fn doIO(&mut self, addr: u16, val: u8) -> u8
    {
       debug!("doIO({:x}, {:x})", addr, val);
       let slot = (addr >> 4) & 0x0f;
       match slot {
          0	=> self.kbdlatch,			// keyboard
          1	=> { self.clearStrobe(); self.noise() }		// reset kbd strobe
          3	=> { self.soundstate = !self.soundstate; self.noise() }		// speaker
          5	=> { if ((addr & 0x0f) < 8) { self.setGrSwitch(addr); } else { self.setAnnunciator(addr); } self.noise() }
          6	=> self.fakeJoystick(addr),
          7	if (addr == 0xc070) => self.noise() | 0x80, // joystick reset
          8	=> { self.doLanguageCardIO(addr); self.noise() }
          9..15 => match self.slots[slot-8] {
             None => self.noise(),
             Some(ref mut p) => p.doIO(addr, val)
             },
          _	=> self.noise()
      }
   }
   
   fn clearStrobe(&mut self)
   {
      self.kbdlatch &= 0x7f;
      debug!("Clear strobe");
   }

   pub fn keyPressed(&mut self, keycode: u8)
   {
      let mut key = (keycode | 0x80) & 0xff;
      // since we're an Apple II+, we don't do lowercase
      if (key >= 0xe1 && key <= 0xfa) { key -= 0x20; }
      self.kbdlatch = key;
      debug!("Key pressed: {}", key);
   }

   fn doLanguageCardIO(&mut self, addr:u16)
   {
      self.aux = match addr & 0xf
      {
       // Select aux RAM bank 2, write protected.
       0|4 => LangCardState::new(true, 2, true),
       // Select ROM, write enable aux RAM bank 2.
       1|5 => LangCardState::new(false, 2, false),
       // Select ROM, write protect aux RAM (either bank).
       2|6|10|14 => LangCardState::new(false, self.aux.auxRAMbank, true),
       // Select aux RAM bank 2, write enabled.
       3|7 => LangCardState::new(true, 2, false),
       // Select aux RAM bank 1, write protected.
       8|12 => LangCardState::new(true, 1, false), 
       // Select ROM, write enable aux RAM bank 1.
       9|13 => LangCardState::new(false, 1, false), 
       // Select aux RAM bank 1, write enabled.
       11|15 => LangCardState::new(true, 1, false),
       // TODO: shouldn't need this
       _ => fail!()
      }
   }
   
   pub fn read_roms(&mut self)
   {
      use std::io::File;
      use std::vec::bytes::copy_memory;
      let ap2rom = File::open(&Path::new("apple2.rom")).read_bytes(0x3000);
      copy_memory(self.mem.mut_slice(0xd000, 0xd000+0x3000), ap2rom);
      info!("loaded apple2.rom");
   }
}
