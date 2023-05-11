use crate::cpu::Mem;
use crate::cartridge::Rom;
const RAM: u16 = 0x0000;
const RAM_END: u16 = 0x1FFF;
const PPU_REG: u16 = 0x2000;
const PPU_REG_END: u16 = 0x3FFF;

pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: Rom,
}

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_vram: [0; 2048],
            rom: rom,
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            //mirror if needed
            addr = addr % 0x4000;
        }
        self.rom.prg_rom[addr as usize]
    }
}

impl Mem for Bus {
    fn mem_read(&self, address: u16) -> u8 {
        match address {
            RAM ..= RAM_END => {
                let mir_down_address = address & 0b0000011111111111;
                self.cpu_vram[mir_down_address as usize]
            }

            PPU_REG ..= PPU_REG_END => {
                let mir_down_address = address & 0b0010000000000111;
                todo!("Impl PPU")
            }

            0x8000 ..= 0xFFFF => self.read_prg_rom(address),

            _ => {
                println!("Ignoring memory access at {}", address);
                0
            }
        }
    }

    fn mem_write(&mut self, address: u16, data: u8) {
        match address {
            RAM ..= RAM_END => {
                let mir_down_address = address & 0b0000011111111111;
                self.cpu_vram[mir_down_address as usize] = data;
            }

            PPU_REG ..= PPU_REG_END => {
                let mir_down_address = address & 0b0010000000000111;
                todo!("Impl PPU")
            }

            0x8000 ..= 0xFFFF => {
                panic!("Do not write on ROM space !!")
            }

            _ => {
                println!("Ignoring memory access at {}", address);
            }
        }
    }
}