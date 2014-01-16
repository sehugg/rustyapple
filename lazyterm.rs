
use std::vec;
use std::io;
use std::io::stdio::{StdReader,StdWriter};
use std::io::Timer;

// TODO: use these but they are inconvenient
pub struct TermColor(u8);

#[deriving(Clone)]
pub struct TermCell
{
  bg: u8,
  fg: u8,
  ch: char
}

//static BLACK: TermColor = TermColor(0);
//static WHITE: TermColor = TermColor(15);
pub static BLACK: u8 = 0;
pub static WHITE: u8 = 15;

pub static EMPTY: TermCell = TermCell { fg:WHITE, bg:BLACK, ch:' ' };

#[deriving(Clone)]
pub struct Buffer
{
  buf: ~[~[TermCell]],
  width: uint,
  height: uint,
}

impl Buffer
{
  pub fn new(cols: uint, rows: uint) -> Buffer
  {
    Buffer { width:cols, height:rows, buf:vec::from_elem(rows, vec::from_elem(cols, EMPTY)) }
  }
  
  pub fn set(&mut self, col: uint, row: uint, cell: TermCell)
  {
    self.buf[row][col] = cell;
  }
}

pub struct Terminal
{
  hin : StdReader,
  hout: StdWriter,
  lastbuf: Buffer,
}

impl Terminal
{
  pub fn new() -> Terminal
  {
    Terminal { lastbuf: Buffer::new(0,0), hin: io::stdin(), hout: io::stdout() }
  }
  
  fn reset(&mut self)
  {
    self.hout.write_str(format!("\x1b[0"));
  }
  
  fn update(&mut self, buf: &Buffer)
  {
      let ref mut hout = self.hout;
      let mut prev = EMPTY;
      for y in range(0,buf.height)
      {
        for x in range(0,buf.width)
        {
          let cell = buf.buf[y][x];
          let last = self.lastbuf.buf[y][x];
          let mut dirty = false;
          if (cell.fg != prev.fg)
          {
            hout.write_str(format!("\x1b[38;5;{}m", cell.fg));
            dirty = true;
          }
          if (cell.bg != prev.bg)
          {
            hout.write_str(format!("\x1b[48;5;{}m", cell.bg));
            dirty = true;
          }
          // TODO
          //if (dirty || cell.ch != last.ch)
          {
            hout.write_char(cell.ch);
          }
          prev = cell;
        }
        hout.write_char('\n');
      }
  }

  fn redraw(&mut self, buf: &Buffer)
  {
      let ref mut hout = self.hout;
      for y in range(0,buf.height)
      {
        for x in range(0,buf.width)
        {
          let cell = buf.buf[y][x];
          hout.write_str(format!("\x1b[38;5;{}m", cell.fg));
          hout.write_str(format!("\x1b[48;5;{}m", cell.bg));
          hout.write_char(cell.ch);
        }
        hout.write_char('\n');
      }
  }

  pub fn refresh(&mut self, buf: &Buffer)
  {
    self.reset();
    if buf.width == self.lastbuf.width && buf.height == self.lastbuf.height
    {
      // scroll up N lines
      self.hout.write_str(format!("\x1b[{}A", buf.height));
      // update dirty cells
      self.update(buf);
    } else {
      // redraw entire window
      // TODO: rescroll window
      self.redraw(buf);
    }
    self.reset();
    self.lastbuf = buf.clone();
  }
}

//

fn main()
{
  let mut buf = Buffer::new(40,24);
  let mut term = Terminal::new();
  let mut timer = Timer::new().unwrap();
  for i in range(0u8,15)
  {
    buf.set(1, 1, TermCell { bg:i, fg:i+1, ch:'#' } );
    term.refresh(&buf);
    timer.sleep(50);
  }
}
