
#[feature(link_args, macro_rules)];

use cpu::Cpu;
use mem::Mem;
use a2::AppleII;
use a2::Peripheral;
use diskii::DiskController;
use util::current_time_millis;
use lazyterm::{Terminal,Buffer};

// NB: This must be first to pick up the macro definitions. What a botch.
#[macro_escape]
pub mod util;

#[macro_escape]
pub mod cpu;
pub mod mem;
pub mod a2;
pub mod diskii;

pub mod lazyterm;

static text_lut: [u16, ..8*3] = [
   0x000, 0x080, 0x100, 0x180, 0x200, 0x280, 0x300, 0x380,
   0x028, 0x0a8, 0x128, 0x1a8, 0x228, 0x2a8, 0x328, 0x3a8,
   0x050, 0x0d0, 0x150, 0x1d0, 0x250, 0x2d0, 0x350, 0x3d0
];

static flashInterval: u64 = 500;

fn draw_text_line(a2: &AppleII, buf: &mut Buffer, flash: bool, y: uint)
{
  // get the base address of this line
  let base = text_lut[y] + if (a2.grswitch & a2::GR_PAGE1) != 0 { 0x800 } else { 0x400 };
  for x in range(0u,40)
  {
     let mut b = a2.mem[base + x as u16];
     let invert: bool;
     // invert flash characters 1/2 of the time
     if (b >= 0x80)
     {
        invert = false;
     } 
     else if (b >= 0x40)
     {
        invert = flash;
        if (flash) { b -= 0x40; } else { b += 0x40; }
     }
     else
     {
        invert = true;
      }
     // if the char. changed, draw it
     let ch = (b & 0x7f) as char;
     let mut cell = if !invert {
       lazyterm::TermCell { bg:lazyterm::BLACK, fg:lazyterm::WHITE, ch:ch }
     } else {
       lazyterm::TermCell { bg:lazyterm::WHITE, fg:lazyterm::BLACK, ch:ch }
     };
     buf.set(x*2, y, cell);
     cell.ch = ' ';
     buf.set(x*2+1, y, cell);
  }
}

fn update_term_buf(a2: &AppleII, buf: &mut Buffer, flash: bool)
{
  for y in range(0u,24)
  {
    draw_text_line(a2, buf, flash, y);
  }
}

fn main()
{
    let mut a2 = AppleII::new();
    a2.read_roms();
    let mut dc: DiskController = DiskController::new();
    dc.load_disk(0, "JUNK4.DSK");
    assert!(dc.has_disk(0));
    a2.set_slot(6, ~dc);
    let mut cpu = Cpu::new(a2);
    cpu.reset();
    
    let mut term = Terminal::new();
    let mut buf = Buffer::new(80,24);
    
    // mismatched types: expected `<generic integer #5>` but found `<generic float #0>`
    let speedup = 2;
    let clocks_per_msec = (1000 * speedup);
    let mut t0 = current_time_millis();
    loop
    {
        // cursor flashing?
        let flash = (t0 % (flashInterval<<1)) > flashInterval;
        update_term_buf(&cpu.mem, &mut buf, flash);
        term.refresh(&buf);
        
        let t1 = current_time_millis();
        let cycle = cpu.cy + (t1-t0)*clocks_per_msec;
        while cpu.cy < cycle
        {
          cpu.step();
        }
        t0 = t1;
    }
}

