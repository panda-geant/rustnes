use std::collections::HashMap;
use crate::opcodes;

bitflags! {

    pub struct Flags: u8 {
        const CARRY = 0b00000001;
        const ZERO  = 0b00000010;
        const INTERRUPT = 0b00000100;
        const DECIMAL = 0b00001000;
        const BREAK = 0b00010000;
        const BREAKBIS = 0b00100000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = 0b10000000;
    }

}

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: Flags,
    pub stack_pointer: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF]
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

trait Mem {
    fn mem_read(&self, addr: u16) -> u8; 

    fn mem_write(&mut self, addr: u16, data: u8);
    
    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

impl Mem for CPU {
    
    fn mem_read(&self, addr: u16) -> u8 { 
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) { 
        self.memory[addr as usize] = data;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: Flags::from_bits_truncate(0b100100),
            stack_pointer: 0,
            program_counter: 0,
            memory: [0; 0xFFFF]
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {

        match mode {
            AddressingMode::Immediate => self.program_counter,

            AddressingMode::ZeroPage  => self.mem_read(self.program_counter) as u16,
            
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
          
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }

            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }

            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }
           
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }

    }

    fn set_a(&mut self, data: u8) {
        self.register_a = data;
        self.update_z_n_flags(self.register_a);
    }

    fn add_to_a(&mut self, data: u8) {

        let sum = self.register_a as u16
            + data as u16 
            + (if self.status.contains(Flags::CARRY) { // This condition because CARRY flag used when overflow during arithmetic operation
                1
            } else {
                0
            }) as u16;

            let carry = sum > 0xff;

            if carry {
                self.status.insert(Flags::CARRY);
            } else {
                self.status.remove(Flags::CARRY);
            }

            let res = sum as u8;

            if res ^ data & res ^ self.register_a ^ 0b10000000 != 0 {
                self.status.insert(Flags::OVERFLOW);
            } else {
                self.status.remove(Flags::OVERFLOW);
            }

            self.set_a(res);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.add_to_a(value);
    }

    fn and(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        let value = self.mem_read(address);
        self.set_a(value & self.register_a);
    }

    fn asl_acc(&mut self) {
        let mut data = self.register_a;

        if data >> 7 == 1 {
            self.status.insert(Flags::CARRY);
        } else {
            self.status.remove(Flags::CARRY);
        }

        data = data << 1;
        self.set_a(data);
    }

    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let address = self.get_operand_address(mode);
        let mut data = self.mem_read(address);
        if data >> 7 == 1 {
            self.status.insert(Flags::CARRY);
        } else {
            self.status.remove(Flags::CARRY);
        }

        data = data << 1;
        self.mem_write(address, data);
        self.update_z_n_flags(data);
        data
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_z_n_flags(self.register_a);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_z_n_flags(self.register_x);
    }

    fn update_z_n_flags(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(Flags::ZERO);
        } else {
            self.status.remove(Flags::ZERO);
        }

        if result & 0b1000_0000 != 0 {
            self.status.insert(Flags::NEGATIVE);
        } else {
            self.status.remove(Flags::NEGATIVE);
        }
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_z_n_flags(self.register_x);
    }

    fn b(&mut self, cond: bool) {
        if cond {
            let curr_at_counter = self.mem_read(self.program_counter) as i8;
            let address = self.program_counter.wrapping_add(1).wrapping_add(curr_at_counter as u16);

            self.program_counter = address;
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        let data = self.mem_read(address);

        if self.register_a & data == 0 {
            self.status.insert(Flags::ZERO);
        } else {
            self.status.remove(Flags::ZERO);
        }

        self.status.set(Flags::NEGATIVE , data & 0b10000000 > 0);
        self.status.set(Flags::OVERFLOW , data & 0b01000000 > 0);
    }
    
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = Flags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;

            let opcode = opcodes.get(&code).expect(&format!("OpCode {:x} is not recognized", code));

            match code {
                /* ADC */
                0x69 | 0x65 | 0x75 | 0x6D | 0x7D | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode);
                }

                /* AND */
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    self.and(&opcode.mode);
                }

                0x0a => self.asl_acc(),

                /* ASL */
                0x06 | 0x16 | 0x0e | 0x1e => {
                    self.asl(&opcode.mode);
                }

                /* LDA */
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&opcode.mode);
                }

                /* STA */
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                0x90 => self.b(!self.status.contains(Flags::CARRY)),
                0xb0 => self.b(self.status.contains(Flags::CARRY)),
                0xf0 => self.b(self.status.contains(Flags::ZERO)),
                0x30 => self.b(self.status.contains(Flags::NEGATIVE)),
                0xd0 => self.b(!self.status.contains(Flags::ZERO)),
                0x10 => self.b(!self.status.contains(Flags::NEGATIVE)),
                0x50 => self.b(!self.status.contains(Flags::OVERFLOW)),
                0x70 => self.b(self.status.contains(Flags::OVERFLOW)),
                
                0x24 | 0x2c => self.bit(&opcode.mode),
                
                0xAA => self.tax(),
                0xe8 => self.inx(),
                0x00 => return,
                _ => todo!(),
            }

            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0A,0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }
}

