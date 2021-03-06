
use cpu::Cpu;
use mem::Mem;
use a2::AppleII;
use a2::Peripheral;
use diskii::DiskController;

//

fn return_two() -> int {
    2
}

#[test]
fn return_two_test() {
    let x = return_two();
    assert!(x == 2);
}

pub struct Ram { mem: [u8, ..0x800] }

impl Mem for Ram {
    fn loadb(&mut self, addr: u16) -> u8     { self.mem[addr & 0x7ff] }
    fn storeb(&mut self, addr: u16, val: u8) { self.mem[addr & 0x7ff] = val }
}

#[test]
fn test_cpu()
{
    let ram = Ram { mem: [ 0, ..0x800 ] };
    let mut cpu = Cpu::new(ram);
    cpu.reset();
    assert!(cpu.regs.pc == 0);
    cpu.step();
}

#[test]
fn test_a2()
{
    let mut a2 = AppleII::new();
    a2.read_roms();
    let mut dc: DiskController = DiskController::new();
    dc.load_disk(0, "JUNK4.DSK");
    assert!(dc.has_disk(0));
    a2.set_slot(6, ~dc);
    let mut cpu = Cpu::new(a2);
    cpu.reset();
    for i in range(0,10*1000000) {
        cpu.step();
    }
}

