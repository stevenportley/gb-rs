use crate::{bus::Bus, interrupts::IntSource};
use std::io;

fn does_bit3_overflow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;

    return (0xF - a) < b;
}

fn does_bit11_overflow(a: u16, b: u16) -> bool {
    let a = a & 0xFFF;
    let b = b & 0xFFF;

    return (0xFFF - a) < b;
}

fn does_bit3_borrow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;
    return b > a;
}

pub struct Cpu<B: Bus> {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    z_f: bool,
    n_f: bool,
    h_f: bool,
    c_f: bool,

    ime: bool,

    pub sleep: bool,
    pub bus: B,
}

#[derive(Debug)]
enum Operands {
    A,  // The accumulator
    SP, // The stadck pointer
    HL, // HL register
    R8(u8),
    R16(u8),
    R16Stk(u8),
    R16Mem(u8),
    Cond(u8), // TODO(SP): Figure out what this is. https://gbdev.io/pandocs/CPU_Instruction_Set.html#cb-prefix-instructions
    B3(u8),   // TODO(SP): Figure out what this is.
    Tgt3(u8), //TODO(SP): Figure out what this is.
    Imm8(u8),
    Imm16(u16), //TODO(SP): This needs to be little-endian, make sure to test this
    SpImm8(u8), // This is SP + Imm8
}

const PAGE0_OFFSET: u16 = 0xFF00;

// Bit indices to address particular register
const A_REG: u8 = 7;
const C_REG: u8 = 1;
const HL_REG: u8 = 2;

const HL_PTR: u8 = 6;

#[derive(Debug, PartialEq)]
enum Opcode {
    NOP,
    LD,
    INC,
    DEC,
    ADD,
    RLCA,
    RRCA,
    RLA,
    RRA,
    DAA,
    CPL,
    SCF,
    CCF,

    JR,
    STOP,
    HALT,
    ADC,
    SUB,
    SBC,
    AND,
    XOR,
    OR,
    CP,
    RET,
    RETI,
    JP,
    CALL,
    RST,
    POP,
    PUSH,
    LDH,
    DI,
    EI,
    //0xCB instructions
    RLC,
    RRC,
    RL,
    RR,
    SLA,
    SRA,
    SWAP,
    SRL,
    BIT,
    RES,
    SET,
}

#[derive(Debug)]
pub struct Instr {
    opcode: Opcode,
    op1: Option<Operands>,
    op2: Option<Operands>,
}

impl<B: Bus> Cpu<B> {
    pub fn new(bus: B) -> Self {
        let mut cpu = Cpu {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            h_f: true,
            c_f: true,
            n_f: false,
            z_f: true,
            ime: false,
            sleep: false,
            bus,
        };

        // Temporary to make LCD work with test ROMs
        cpu.bus.write(0xFF44, 0x90);
        cpu
    }

    fn get_f(&self) -> u8 {
        let z = if self.z_f { 0x80 } else { 0 };
        let n = if self.n_f { 0x40 } else { 0 };
        let h = if self.h_f { 0x20 } else { 0 };
        let c = if self.c_f { 0x10 } else { 0 };
        return z | n | h | c;
    }

    fn rreg8(&mut self, dst: u8) -> u8 {
        match dst {
            0 => return self.b,
            1 => return self.c,
            2 => return self.d,
            3 => return self.e,
            4 => return self.h,
            5 => return self.l,
            6 => {
                // This is a special case, instead of setting a register,
                // we use the memory location pointed to by the HL register
                let hl = ((self.h as u16) << 8) | (self.l as u16);
                return self.bus.read(hl);
            }
            7 => return self.a,
            _ => unreachable!("rreg8 with invalid bit index! {dst}"),
        }
    }

    fn wreg8(&mut self, dst: u8, val: u8) {
        match dst {
            0 => self.b = val,
            1 => self.c = val,
            2 => self.d = val,
            3 => self.e = val,
            4 => self.h = val,
            5 => self.l = val,
            6 => {
                // This is a special case, instead of setting a register,
                // we use the memory location pointed to by the HL register
                let hl = ((self.h as u16) << 8) | (self.l as u16);
                self.bus.write(hl, val);
            }
            7 => self.a = val,
            _ => unreachable!("Set reg8 with invalid bit index! {dst}"),
        }
    }

    fn wreg16(&mut self, dst: u8, val: u16) {
        let high: u8 = (val >> 8) as u8;
        let low: u8 = val as u8;

        match dst {
            0 => {
                self.b = high;
                self.c = low;
            }
            1 => {
                self.d = high;
                self.e = low;
            }
            2 => {
                self.h = high;
                self.l = low;
            }
            3 => {
                self.sp = val;
            }
            _ => {
                unreachable!("Set reg16 with invalid bit index! {dst}")
            }
        }
    }

    fn rreg16(&mut self, dst: u8) -> u16 {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };

        match dst {
            0 => return make_u16(self.b, self.c),
            1 => return make_u16(self.d, self.e),
            2 => return make_u16(self.h, self.l),
            3 => return self.sp,
            _ => unreachable!("rreg16 with invalid bit index! {dst}"),
        }
    }

    fn rr16mem(&mut self, r16mem: u8) -> u8 {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };
        match r16mem {
            0 => return self.bus.read(make_u16(self.b, self.c)),
            1 => return self.bus.read(make_u16(self.d, self.e)),
            2 => {
                let mut hl = make_u16(self.h, self.l);
                let ret = self.bus.read(hl);
                hl = hl + 1;
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
                return ret;
            }
            3 => {
                let mut hl = make_u16(self.h, self.l);
                let ret = self.bus.read(hl);
                hl = hl.wrapping_sub(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
                return ret;
            }
            _ => unreachable!("rr16mem with invalid bit index! {r16mem}"),
        }
    }

    fn wr16mem(&mut self, r16mem: u8, val: u8) {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };
        match r16mem {
            0 => self.bus.write(make_u16(self.b, self.c), val),
            1 => self.bus.write(make_u16(self.d, self.e), val),
            2 => {
                let mut hl = make_u16(self.h, self.l);
                self.bus.write(hl, val);
                hl = hl.wrapping_add(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
            }
            3 => {
                let mut hl = make_u16(self.h, self.l);
                self.bus.write(hl, val);
                hl = hl.wrapping_sub(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
            }
            _ => unreachable!("rr16mem with invalid bit index! {r16mem}"),
        }
    }

    fn push_stack(&mut self, val: u16) {
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val >> 8) as u8);
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val & 0xFF) as u8);
    }

    fn pop_stack(&mut self) -> u16 {
        let mut ret = self.bus.read(self.sp) as u16;
        self.sp = self.sp + 1;
        ret |= (self.bus.read(self.sp) as u16) << 8;
        self.sp = self.sp + 1;
        return ret;
    }

    fn rlc(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let msb = reg_val >> 7;
        let new_val = (reg_val << 1) | msb;
        self.c_f = msb == 1;
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.wreg8(reg, (reg_val << 1) | msb);
    }

    fn rl(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let msb = reg_val >> 7;
        let new_lsb = if self.c_f { 1 } else { 0 };
        let new_val = (reg_val << 1) | new_lsb;

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = msb == 1;
    }

    fn rrc(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let lsb = reg_val & 1;
        let new_val = (reg_val >> 1) | (lsb << 7);

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = lsb == 1;
    }

    fn rr(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let new_msb = if self.c_f { 0x80 } else { 0 };
        let new_val = (reg_val >> 1) | new_msb;

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = (reg_val & 1) == 1;
    }

    fn check_cond(&self, cond: u8) -> bool {
        match cond {
            0 => return !self.z_f,
            1 => return self.z_f,
            2 => return !self.c_f,
            3 => return self.c_f,
            _ => unreachable!("Invalid condition check! {cond}"),
        }
    }

    fn load_byte(&mut self) -> u8 {
        let next_byte = self.bus.read(self.pc);
        self.pc += 1;
        return next_byte;
    }

    fn load_word(&mut self) -> u16 {
        let next = self.load_byte();
        let next_next = self.load_byte();

        return (next as u16) | ((next_next as u16) << 8);
    }

    fn block0_decode(&mut self, opcode: u8) -> Instr {
        let lower_four = opcode & 0xF;
        let lower_three = opcode & 0x7;
        let is_jr_cond = (opcode & 0xE7) == 0x20; //Is this opcode for `jr cond, imm8`

        let instr = {
            // NOP
            if opcode == 0 {
                Instr {
                    opcode: Opcode::NOP,
                    op1: None,
                    op2: None,
                }

            // LD
            } else if lower_four == 1 {
                let op1 = (opcode >> 4) & 0x3;
                let op2 = self.load_word();
                Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::R16(op1)),
                    op2: Some(Operands::Imm16(op2)),
                }
            } else if lower_four == 2 {
                let op1 = (opcode >> 4) & 0x3;
                Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::R16Mem(op1)),
                    op2: Some(Operands::A),
                }
            } else if lower_four == 10 {
                let op2 = (opcode >> 4) & 0x3;
                Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::A),
                    op2: Some(Operands::R16Mem(op2)),
                }
            } else if lower_four == 3 {
                let op1 = (opcode >> 4) & 0x3;
                Instr {
                    opcode: Opcode::INC,
                    op1: Some(Operands::R16(op1)),
                    op2: None,
                }
            } else if lower_four == 0xB {
                let op1 = (opcode >> 4) & 0x3;
                Instr {
                    opcode: Opcode::DEC,
                    op1: Some(Operands::R16(op1)),
                    op2: None,
                }
            } else if lower_four == 0x9 {
                let op2 = (opcode >> 4) & 0x3;
                Instr {
                    opcode: Opcode::ADD,
                    op1: Some(Operands::HL),
                    op2: Some(Operands::R16(op2)),
                }
            } else if opcode == 0x8 {
                let op1 = self.load_word();
                Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::Imm16(op1)),
                    op2: Some(Operands::SP),
                }
            } else if lower_three == 0x4 {
                let op1 = (opcode >> 3) & 0x7;
                Instr {
                    opcode: Opcode::INC,
                    op1: Some(Operands::R8(op1)),
                    op2: None,
                }
            } else if lower_three == 5 {
                let op1 = (opcode >> 3) & 0x7;
                Instr {
                    opcode: Opcode::DEC,
                    op1: Some(Operands::R8(op1)),
                    op2: None,
                }
            } else if lower_three == 6 {
                let op1 = (opcode >> 3) & 0x7;
                let op2 = self.load_byte();
                Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::R8(op1)),
                    op2: Some(Operands::Imm8(op2)),
                }
            } else if opcode == 0x7 {
                Instr {
                    opcode: Opcode::RLCA,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0xF {
                Instr {
                    opcode: Opcode::RRCA,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x17 {
                Instr {
                    opcode: Opcode::RLA,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x1F {
                Instr {
                    opcode: Opcode::RRA,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x27 {
                Instr {
                    opcode: Opcode::DAA,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x2F {
                Instr {
                    opcode: Opcode::CPL,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x37 {
                Instr {
                    opcode: Opcode::SCF,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x3F {
                Instr {
                    opcode: Opcode::CCF,
                    op1: None,
                    op2: None,
                }
            } else if opcode == 0x18 {
                let next_byte = self.load_byte();
                Instr {
                    opcode: Opcode::JR,
                    op1: Some(Operands::Imm8(next_byte)),
                    op2: None,
                }
            } else if is_jr_cond {
                let next_byte = self.load_byte();
                let cond = (opcode >> 3) & 0x3;
                Instr {
                    opcode: Opcode::JR,
                    op1: Some(Operands::Cond(cond)),
                    op2: Some(Operands::Imm8(next_byte)),
                }
            } else if opcode == 0x10 {
                Instr {
                    opcode: Opcode::STOP,
                    op1: None,
                    op2: None,
                }
            } else {
                unreachable!(
                    "Block0 opcode decoding failed. Illegal opcode: {:#04x}",
                    opcode
                );
            }
        };

        return instr;
    }
    fn block1_decode(&mut self, opcode: u8) -> Instr {
        if opcode == 0x76 {
            return Instr {
                opcode: Opcode::HALT,
                op1: None,
                op2: None,
            };
        }

        if (opcode & 0xC0) != 0x40 {
            unreachable!(
                "Block1 opcode decoding failed. Illegal opcode: {:#04x}",
                opcode
            );
        }

        let src = opcode & 0x7;
        let dst = (opcode >> 3) & 0x7;

        return Instr {
            opcode: Opcode::LD,
            op1: Some(Operands::R8(dst)),
            op2: Some(Operands::R8(src)),
        };
    }

    fn block2_decode(&mut self, opcode: u8) -> Instr {
        let code = opcode & 0xF8; // Clear lower 3 bits
        let operand = opcode & 0x7;

        let this_opcode = match code {
            0x80 => Opcode::ADD,
            0x88 => Opcode::ADC,
            0x90 => Opcode::SUB,
            0x98 => Opcode::SBC,
            0xA0 => Opcode::AND,
            0xA8 => Opcode::XOR,
            0xB0 => Opcode::OR,
            0xB8 => Opcode::CP,
            _ => unreachable!(
                "Block2 opcode decoding failed. Illegal opcode: {:#04x}",
                opcode
            ),
        };

        return Instr {
            opcode: this_opcode,
            op1: Some(Operands::A),
            op2: Some(Operands::R8(operand)),
        };
    }
    fn block3_decode(&mut self, opcode: u8) -> Instr {
        let lower_three = opcode & 0x7;
        if lower_three == 0x6 {
            //One of the immediates in block3
            let next_byte = self.load_byte();
            let this_code = match opcode {
                0xC6 => Opcode::ADD,
                0xCE => Opcode::ADC,
                0xD6 => Opcode::SUB,
                0xDE => Opcode::SBC,
                0xE6 => Opcode::AND,
                0xEE => Opcode::XOR,
                0xF6 => Opcode::OR,
                0xFE => Opcode::CP,
                _ => unreachable!(
                    "Block3 opcode decoding failed. Illegal opcode: {:#04x}",
                    opcode
                ),
            };

            return Instr {
                opcode: this_code,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(next_byte)),
            };
        }

        // One of the instructions in block 3
        // below 0xCB prefix
        match opcode {
            0xE2 => {
                return Instr {
                    opcode: Opcode::LDH,
                    op1: Some(Operands::R8(C_REG)),
                    op2: Some(Operands::A),
                }
            }
            0xE0 => {
                let next_byte = self.load_byte();
                return Instr {
                    opcode: Opcode::LDH,
                    op1: Some(Operands::Imm8(next_byte)),
                    op2: Some(Operands::A),
                };
            }
            0xEA => {
                let next_word = self.load_word();
                return Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::Imm16(next_word)),
                    op2: Some(Operands::A),
                };
            }

            0xF2 => {
                return Instr {
                    opcode: Opcode::LDH,
                    op1: Some(Operands::A),
                    op2: Some(Operands::R8(C_REG)),
                };
            }

            0xF0 => {
                let next_byte = self.load_byte();
                return Instr {
                    opcode: Opcode::LDH,
                    op1: Some(Operands::A),
                    op2: Some(Operands::Imm8(next_byte)),
                };
            }

            0xFA => {
                let next_word = self.load_word();
                return Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::A),
                    op2: Some(Operands::Imm16(next_word)),
                };
            }

            0xE8 => {
                let next_byte = self.load_byte();
                return Instr {
                    opcode: Opcode::ADD,
                    op1: Some(Operands::SP),
                    op2: Some(Operands::Imm8(next_byte)),
                };
            }

            0xF8 => {
                let next_byte = self.load_byte();
                return Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::HL),
                    op2: Some(Operands::SpImm8(next_byte)),
                };
            }

            0xF9 => {
                return Instr {
                    opcode: Opcode::LD,
                    op1: Some(Operands::SP),
                    op2: Some(Operands::HL),
                };
            }

            0xF3 => {
                return Instr {
                    opcode: Opcode::DI,
                    op1: None,
                    op2: None,
                };
            }

            0xFB => {
                return Instr {
                    opcode: Opcode::EI,
                    op1: None,
                    op2: None,
                };
            }

            _ => {}
        }

        if opcode == 0xCB {
            let next_byte = self.load_byte();
            return Self::cb_prefix_decode(next_byte);
        }

        let lower_four = opcode & 0xF;
        if lower_four == 1 {
            let r16stk = (opcode >> 4) & 0x3;
            return Instr {
                opcode: Opcode::POP,
                op1: Some(Operands::R16Stk(r16stk)),
                op2: None,
            };
        }

        if lower_four == 5 {
            let r16stk = (opcode >> 4) & 0x3;
            return Instr {
                opcode: Opcode::PUSH,
                op1: Some(Operands::R16Stk(r16stk)),
                op2: None,
            };
        }

        let bit43 = (opcode >> 3) & 0x3;

        if lower_three == 0 {
            return Instr {
                opcode: Opcode::RET,
                op1: Some(Operands::Cond(bit43)),
                op2: None,
            };
        }

        if opcode == 0xC9 {
            return Instr {
                opcode: Opcode::RET,
                op1: None,
                op2: None,
            };
        }

        if opcode == 0xD9 {
            return Instr {
                opcode: Opcode::RETI,
                op1: None,
                op2: None,
            };
        }

        if lower_three == 0x2 {
            let next_word = self.load_word();
            return Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::Cond(bit43)),
                op2: Some(Operands::Imm16(next_word)),
            };
        }

        if opcode == 0xC3 {
            let next_word = self.load_word();
            return Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::Imm16(next_word)),
                op2: None,
            };
        }

        if opcode == 0xE9 {
            return Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::HL),
                op2: None,
            };
        }

        if lower_three == 0x4 {
            let bit43 = (opcode >> 3) & 0x3;
            let next_word = self.load_word();
            return Instr {
                opcode: Opcode::CALL,
                op1: Some(Operands::Cond(bit43)),
                op2: Some(Operands::Imm16(next_word)),
            };
        }

        if opcode == 0xCD {
            let next_word = self.load_word();
            return Instr {
                opcode: Opcode::CALL,
                op1: Some(Operands::Imm16(next_word)),
                op2: None,
            };
        }

        if lower_three == 0x7 {
            let bit543 = (opcode >> 3) & 0x7;
            return Instr {
                opcode: Opcode::RST,
                op1: Some(Operands::Tgt3(bit543)),
                op2: None,
            };
        }

        unreachable!(
            "Block3 opcode decoding failed. Illegal opcode: {:#04x}",
            opcode
        );
    }

    fn cb_prefix_decode(opcode: u8) -> Instr {
        let masked = opcode >> 3;
        let lower_three = opcode & 0x7;
        let bit76 = opcode >> 6;
        match masked {
            0 => {
                return Instr {
                    opcode: Opcode::RLC,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            1 => {
                return Instr {
                    opcode: Opcode::RRC,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            2 => {
                return Instr {
                    opcode: Opcode::RL,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            3 => {
                return Instr {
                    opcode: Opcode::RR,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            4 => {
                return Instr {
                    opcode: Opcode::SLA,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            5 => {
                return Instr {
                    opcode: Opcode::SRA,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            6 => {
                return Instr {
                    opcode: Opcode::SWAP,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            7 => {
                return Instr {
                    opcode: Opcode::SRL,
                    op1: Some(Operands::R8(lower_three)),
                    op2: None,
                };
            }
            _ => {}
        };

        let bit543 = (opcode >> 3) & 0x7;
        match bit76 {
            1 => {
                return Instr {
                    opcode: Opcode::BIT,
                    op1: Some(Operands::B3(bit543)),
                    op2: Some(Operands::R8(lower_three)),
                };
            }
            2 => {
                return Instr {
                    opcode: Opcode::RES,
                    op1: Some(Operands::B3(bit543)),
                    op2: Some(Operands::R8(lower_three)),
                };
            }
            3 => {
                return Instr {
                    opcode: Opcode::SET,
                    op1: Some(Operands::B3(bit543)),
                    op2: Some(Operands::R8(lower_three)),
                };
            }
            _ => {}
        }

        unreachable!("Invalid opcode: {opcode}");
    }

    pub fn next_instr(&mut self) -> Instr {
        let opcode = self.bus.read(self.pc);
        self.pc += 1;

        let instr = match opcode {
            0x00..=0x3F => self.block0_decode(opcode),
            0x40..=0x7F => self.block1_decode(opcode),
            0x80..=0xBF => self.block2_decode(opcode),
            0xC0..=0xFF => self.block3_decode(opcode),
        };

        instr
    }

    pub fn log_state(&self) {
        // This is formatted for use with gb doctor
        // https://robertheaton.com/gameboy-doctor/
        println!("A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
            self.a,
            self.get_f(),
            self.b,
            self.c,
            self.d,
            self.e,
            self.h,
            self.l,
            self.sp,
            self.pc,
            self.bus.read(self.pc),
            self.bus.read(self.pc + 1),
            self.bus.read(self.pc + 2),
            self.bus.read(self.pc + 3));
    }

    pub fn is_passed(&self) -> bool {
        return self.bus.is_passed();
    }

    pub fn execute_instr(&mut self, instr: Instr) -> usize {
        match instr {
            Instr {
                opcode: Opcode::NOP,
                op1: None,
                op2: None,
            } => 1,
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::R16(r1)),
                op2: Some(Operands::Imm16(i)),
            } => {
                self.wreg16(r1, i);
                3
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::R16Mem(mem)),
                op2: Some(Operands::A),
            } => {
                self.wr16mem(mem, self.a);
                2
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::A),
                op2: Some(Operands::R16Mem(mem)),
            } => {
                self.a = self.rr16mem(mem);
                return 2;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::Imm16(i)),
                op2: Some(Operands::SP),
            } => {
                // This is why: https://rgbds.gbdev.io/docs/v0.8.0/gbz80.7#LD__n16_,SP
                self.bus.write(i, self.sp as u8);
                self.bus.write(i + 1, (self.sp >> 8) as u8);
                return 5;
            }
            Instr {
                opcode: Opcode::INC,
                op1: Some(Operands::R16(reg)),
                op2: None,
            } => {
                let plus_one = self.rreg16(reg).wrapping_add(1);
                self.wreg16(reg, plus_one);

                return 2;
            }
            Instr {
                opcode: Opcode::DEC,
                op1: Some(Operands::R16(reg)),
                op2: None,
            } => {
                let minus_one = self.rreg16(reg).wrapping_sub(1);
                self.wreg16(reg, minus_one);
                return 2;
            }
            Instr {
                opcode: Opcode::ADD,
                op1: Some(Operands::HL),
                op2: Some(Operands::R16(reg)),
            } => {
                let hl_val = self.rreg16(HL_REG);
                let reg_val = self.rreg16(reg);
                let (new_val, overflow) = hl_val.overflowing_add(reg_val);
                self.wreg16(HL_REG, new_val);
                self.n_f = false;
                self.h_f = does_bit11_overflow(hl_val, reg_val);
                self.c_f = overflow;
                return 2;
            }
            Instr {
                opcode: Opcode::INC,
                op1: Some(Operands::R8(reg)),
                op2: None,
            } => {
                let before = self.rreg8(reg);
                self.h_f = does_bit3_overflow(before, 1);
                self.n_f = false;
                let incre = before.wrapping_add(1);
                self.z_f = incre == 0;
                self.wreg8(reg, incre);

                return if reg == HL_PTR { 3 } else { 1 };
            }
            Instr {
                opcode: Opcode::DEC,
                op1: Some(Operands::R8(reg)),
                op2: None,
            } => {
                let before = self.rreg8(reg);
                let dec = before.wrapping_sub(1);
                self.wreg8(reg, dec);

                self.z_f = dec == 0;
                self.h_f = does_bit3_borrow(before, 1);
                self.n_f = true;

                return if reg == HL_PTR { 3 } else { 1 };
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::R8(reg)),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.wreg8(reg, i);
                return if reg == HL_PTR { 3 } else { 2 };
            }
            Instr {
                opcode: Opcode::RLCA,
                op1: None,
                op2: None,
            } => {
                self.rlc(A_REG);
                self.z_f = false;
                return 1;
            }
            Instr {
                opcode: Opcode::RRCA,
                op1: None,
                op2: None,
            } => {
                self.rrc(A_REG);
                self.z_f = false;
                return 1;
            }
            Instr {
                opcode: Opcode::RLA,
                op1: None,
                op2: None,
            } => {
                self.rl(A_REG);
                self.z_f = false;
                return 1;
            }
            Instr {
                opcode: Opcode::RRA,
                op1: None,
                op2: None,
            } => {
                self.rr(A_REG);
                self.z_f = false;
                return 1;
            }
            Instr {
                opcode: Opcode::DAA,
                op1: None,
                op2: None,
            } => {
                //https://forums.nesdev.org/viewtopic.php?t=15944

                /*
                       if (!n_flag) {  // after an addition, adjust if (half-)carry occurred or if result is out of bounds
                  if (c_flag || a > 0x99) { a += 0x60; c_flag = 1; }
                  if (h_flag || (a & 0x0f) > 0x09) { a += 0x6; }
                } else {  // after a subtraction, only adjust if (half-)carry occurred
                  if (c_flag) { a -= 0x60; }
                  if (h_flag) { a -= 0x6; }
                }
                // these flags are always updated
                z_flag = (a == 0); // the usual z flag
                h_flag = 0; // h flag is always cleared

                                 *
                                 */
                if self.n_f {
                    if self.c_f {
                        self.a = self.a.wrapping_sub(0x60);
                    }
                    if self.h_f {
                        self.a = self.a.wrapping_sub(0x6);
                    }
                } else {
                    if self.c_f || self.a > 0x99 {
                        self.a = self.a.wrapping_add(0x60);
                        self.c_f = true;
                    }
                    if self.h_f || (self.a & 0x0f) > 0x09 {
                        self.a = self.a.wrapping_add(0x6);
                    }
                }

                self.z_f = self.a == 0;
                self.h_f = false;

                return 1;
            }
            Instr {
                opcode: Opcode::CPL,
                op1: None,
                op2: None,
            } => {
                self.a = !self.a;
                self.n_f = true;
                self.h_f = true;
                return 1;
            }
            Instr {
                opcode: Opcode::SCF,
                op1: None,
                op2: None,
            } => {
                self.n_f = false;
                self.h_f = false;
                self.c_f = true;
                return 1;
            }
            Instr {
                opcode: Opcode::CCF,
                op1: None,
                op2: None,
            } => {
                self.n_f = false;
                self.h_f = false;
                self.c_f = !self.c_f;
                return 1;
            }
            Instr {
                opcode: Opcode::JR,
                op1: Some(Operands::Imm8(i)),
                op2: None,
            } => {
                let offset = i as i8;
                let curr_pc = self.pc as i32;
                let new_pc = curr_pc + offset as i32;
                self.pc = new_pc as u16;
                return 3;
            }

            Instr {
                opcode: Opcode::JR,
                op1: Some(Operands::Cond(cond)),
                op2: Some(Operands::Imm8(i)),
            } => {
                if self.check_cond(cond) {
                    let offset = i as i8;
                    let curr_pc = self.pc as i32;
                    let new_pc = curr_pc + offset as i32;

                    self.pc = new_pc as u16;
                    return 3;
                }
                return 2;
            }

            Instr {
                opcode: Opcode::STOP,
                op1: None,
                op2: None,
            } => {
                unreachable!("Stop instruction not implemented!");
                return 0;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::R8(dst)),
                op2: Some(Operands::R8(src)),
            } => {
                let d = self.rreg8(src);
                self.wreg8(dst, d);
                return if (dst == HL_PTR) || (src == HL_PTR) {
                    2
                } else {
                    1
                };
            }
            Instr {
                opcode: Opcode::HALT,
                op1: None,
                op2: None,
            } => {
                /*
                if self.ime {
                    self.sleep = true;
                }
                */
                self.sleep = true;
                return 1;
                /*
                if !self.bus.int_controller.pending() {
                    self.sleep = true;
                    return 1;
                }
                */

                //TODO: Handle HALT bug
                //assert!(false);
            }
            Instr {
                opcode: Opcode::ADD,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(reg)),
            } => {
                let reg_val = self.rreg8(reg);

                self.h_f = does_bit3_overflow(reg_val, self.a);

                let (new_val, does_overflow) = self.a.overflowing_add(reg_val);
                self.c_f = does_overflow;
                self.z_f = new_val == 0;
                self.n_f = false;
                self.a = new_val;
                return if reg == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::ADC,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);

                self.n_f = false;
                self.h_f = does_bit3_overflow(self.a, reg_val);

                let (added, overflow) = reg_val.overflowing_add(self.a);

                if self.c_f {
                    // Next two conditionals check if the carry will overflow
                    if does_bit3_overflow(added, 1) {
                        self.h_f = true;
                    }

                    self.c_f = overflow || added == 0xFF;
                    self.a = added.wrapping_add(1);
                } else {
                    self.c_f = overflow;
                    self.a = added;
                }

                self.z_f = self.a == 0;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::SUB,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);
                let new_val = self.a.wrapping_sub(reg_val);

                self.h_f = does_bit3_borrow(self.a, reg_val);
                self.c_f = reg_val > self.a;
                self.n_f = true;
                self.z_f = new_val == 0;

                self.a = new_val;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::SBC,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                let carry = if self.c_f { 1 } else { 0 };
                let reg_val = self.rreg8(r);

                self.c_f = (reg_val as u16 + carry as u16) > self.a as u16;

                self.h_f = does_bit3_borrow(self.a, reg_val);

                self.n_f = true;
                self.a = self.a.wrapping_sub(reg_val);

                if does_bit3_borrow(self.a, carry) {
                    self.h_f = true;
                }

                self.a = self.a.wrapping_sub(carry);
                self.z_f = self.a == 0;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::AND,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                self.a = self.a & self.rreg8(r);
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = true;
                self.c_f = false;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::XOR,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                self.a = self.a ^ self.rreg8(r);
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = false;
                self.c_f = false;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::OR,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                self.a = self.a | self.rreg8(r);
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = false;
                self.c_f = false;
                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::CP,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);

                self.h_f = does_bit3_borrow(self.a, reg_val);
                self.c_f = reg_val > self.a;
                self.n_f = true;
                let new_val = self.a.wrapping_sub(reg_val);
                self.z_f = new_val == 0;

                return if r == HL_PTR { 2 } else { 1 };
            }
            Instr {
                opcode: Opcode::ADD,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.h_f = does_bit3_overflow(i, self.a);

                let (new_val, does_overflow) = i.overflowing_add(self.a);
                self.c_f = does_overflow;
                self.z_f = new_val == 0;
                self.n_f = false;
                self.a = new_val;
                return 2;
            }
            Instr {
                opcode: Opcode::ADC,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.n_f = false;
                self.h_f = does_bit3_overflow(self.a, i);

                let (added, overflow) = self.a.overflowing_add(i);

                if self.c_f {
                    // Next two conditionals check if the carry will overflow
                    if does_bit3_overflow(added, 1) {
                        self.h_f = true;
                    }

                    self.c_f = overflow || added == 0xFF;
                    self.a = added.wrapping_add(1);
                } else {
                    self.c_f = overflow;
                    self.a = added;
                }

                self.z_f = self.a == 0;
                return 2;
            }
            Instr {
                opcode: Opcode::SUB,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                let new_val = self.a.wrapping_sub(i);

                self.h_f = does_bit3_borrow(self.a, i);
                self.c_f = i > self.a;
                self.n_f = true;
                self.z_f = new_val == 0;
                self.a = new_val;

                return 2;
            }
            Instr {
                opcode: Opcode::SBC,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                let carry = if self.c_f { 1 } else { 0 };

                self.c_f = (i as u16 + carry as u16) > self.a as u16;

                self.h_f = does_bit3_borrow(self.a, i);

                self.n_f = true;
                self.a = self.a.wrapping_sub(i);

                if does_bit3_borrow(self.a, carry) {
                    self.h_f = true;
                }

                self.a = self.a.wrapping_sub(carry);
                self.z_f = self.a == 0;
                return 2;
            }
            Instr {
                opcode: Opcode::AND,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.a = self.a & i;
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = true;
                self.c_f = false;
                return 2;
            }
            Instr {
                opcode: Opcode::XOR,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.a = self.a ^ i;
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = false;
                self.c_f = false;
                return 2;
            }
            Instr {
                opcode: Opcode::OR,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.a = self.a | i;
                self.z_f = if self.a == 0 { true } else { false };
                self.n_f = false;
                self.h_f = false;
                self.c_f = false;
                return 2;
            }
            Instr {
                opcode: Opcode::CP,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.h_f = does_bit3_borrow(self.a, i);
                self.c_f = i > self.a;

                self.n_f = true;
                let new_val = self.a.wrapping_sub(i);
                self.z_f = new_val == 0;
                return 2;
            }
            Instr {
                opcode: Opcode::RET,
                op1: Some(Operands::Cond(cond)),
                op2: None,
            } => {
                if !self.check_cond(cond) {
                    return 2;
                }

                self.pc = self.pop_stack();
                return 5;
            }
            Instr {
                opcode: Opcode::RET,
                op1: None,
                op2: None,
            } => {
                self.pc = self.pop_stack();
                return 4;
            }
            Instr {
                opcode: Opcode::RETI,
                op1: None,
                op2: None,
            } => {
                self.ime = true;
                self.pc = self.pop_stack();
                return 4;
            }
            Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::Cond(cond)),
                op2: Some(Operands::Imm16(i)),
            } => {
                if self.check_cond(cond) {
                    self.pc = i;
                    return 4;
                }

                return 3;
            }
            Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::Imm16(i)),
                op2: None,
            } => {
                self.pc = i;
                return 4;
            }
            Instr {
                opcode: Opcode::JP,
                op1: Some(Operands::HL),
                op2: None,
            } => {
                let hl = ((self.h as u16) << 8) | self.l as u16;
                self.pc = hl;
                return 1;
            }
            Instr {
                opcode: Opcode::CALL,
                op1: Some(Operands::Cond(cond)),
                op2: Some(Operands::Imm16(i)),
            } => {
                if !self.check_cond(cond) {
                    return 3;
                }

                self.push_stack(self.pc);
                self.pc = i;
                return 6;
            }
            Instr {
                opcode: Opcode::CALL,
                op1: Some(Operands::Imm16(i)),
                op2: None,
            } => {
                self.push_stack(self.pc);
                self.pc = i;
                return 6;
            }
            Instr {
                opcode: Opcode::RST,
                op1: Some(Operands::Tgt3(tgt)),
                op2: None,
            } => {
                self.push_stack(self.pc);
                self.pc = (tgt as u16) << 3;
                return 4;
            }
            Instr {
                opcode: Opcode::POP,
                op1: Some(Operands::R16Stk(r16stk)),
                op2: None,
            } => {
                match r16stk {
                    0 => {
                        let val = self.pop_stack();
                        self.b = (val >> 8) as u8;
                        self.c = (val & 0xFF) as u8;
                    }
                    1 => {
                        let val = self.pop_stack();
                        self.d = (val >> 8) as u8;
                        self.e = (val & 0xFF) as u8;
                    }
                    2 => {
                        let val = self.pop_stack();
                        self.h = (val >> 8) as u8;
                        self.l = (val & 0xFF) as u8;
                    }
                    3 => {
                        let val = self.pop_stack();
                        self.a = (val >> 8) as u8;
                        self.z_f = (val & 0x80) == 0x80;
                        self.n_f = (val & 0x40) == 0x40;
                        self.h_f = (val & 0x20) == 0x20;
                        self.c_f = (val & 0x10) == 0x10;
                    }
                    _ => {
                        unreachable!("Popping from the stack with an invalid r16stk: {:?}", instr);
                    }
                }
                return 3;
            }
            Instr {
                opcode: Opcode::PUSH,
                op1: Some(Operands::R16Stk(r16stk)),
                op2: None,
            } => {
                let val = match r16stk {
                    0 => ((self.b as u16) << 8) | (self.c as u16),
                    1 => ((self.d as u16) << 8) | (self.e as u16),
                    2 => ((self.h as u16) << 8) | (self.l as u16),
                    3 => {
                        let mut val = (self.a as u16) << 8;
                        val = if self.z_f { val | 0x80 } else { val };
                        val = if self.n_f { val | 0x40 } else { val };
                        val = if self.h_f { val | 0x20 } else { val };
                        val = if self.c_f { val | 0x10 } else { val };
                        val
                    }
                    _ => {
                        unreachable!("Popping from the stack with an invalid r16stk: {:?}", instr);
                    }
                };

                self.push_stack(val);
                return 4;
            }
            Instr {
                opcode: Opcode::LDH,
                op1: Some(Operands::R8(C_REG)),
                op2: Some(Operands::A),
            } => {
                self.bus.write(PAGE0_OFFSET + self.c as u16, self.a);
                return 2;
            }
            Instr {
                opcode: Opcode::LDH,
                op1: Some(Operands::Imm8(i)),
                op2: Some(Operands::A),
            } => {
                self.bus.write(i as u16 + PAGE0_OFFSET, self.a);
                return 3;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::Imm16(i)),
                op2: Some(Operands::A),
            } => {
                self.bus.write(i, self.a);
                return 4;
            }
            Instr {
                opcode: Opcode::LDH,
                op1: Some(Operands::A),
                op2: Some(Operands::R8(C_REG)),
            } => {
                self.a = self.bus.read(self.c as u16 + PAGE0_OFFSET);
                return 2;
            }
            Instr {
                opcode: Opcode::LDH,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm8(i)),
            } => {
                self.a = self.bus.read(i as u16 + PAGE0_OFFSET);
                return 3;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::A),
                op2: Some(Operands::Imm16(i)),
            } => {
                self.a = self.bus.read(i);
                return 4;
            }
            Instr {
                opcode: Opcode::ADD,
                op1: Some(Operands::SP),
                op2: Some(Operands::Imm8(i)),
            } => {
                let s_i = i as i8;

                let new_sp = self.sp.wrapping_add(s_i as u16);
                self.z_f = false;
                self.n_f = false;
                self.h_f = does_bit3_overflow(i, self.sp as u8);

                let (_, does_bit7_of) = (self.sp as u8).overflowing_add(i);
                self.c_f = does_bit7_of;

                self.sp = new_sp;
                return 4;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::HL),
                op2: Some(Operands::SpImm8(i)),
            } => {
                let sp = self.sp as i32;
                let i_8 = i as i8;
                let new_hl = (sp + i_8 as i32) as u16;

                self.z_f = false;
                self.n_f = false;
                self.h_f = does_bit3_overflow(i as u8, self.sp as u8);

                // Checking for signed byte add overflow
                let (_, does_of) = (self.sp as u8).overflowing_add(i);

                self.c_f = does_of;
                self.h = (new_hl >> 8) as u8;
                self.l = (new_hl & 0xFF) as u8;
                return 3;
            }
            Instr {
                opcode: Opcode::LD,
                op1: Some(Operands::SP),
                op2: Some(Operands::HL),
            } => {
                let new_sp = ((self.h as u16) << 8) | self.l as u16;
                self.sp = new_sp;
                return 2;
            }
            Instr {
                opcode: Opcode::DI,
                op1: None,
                op2: None,
            } => {
                self.ime = false;
                return 1;
            }
            Instr {
                opcode: Opcode::EI,
                op1: None,
                op2: None,
            } => {
                self.ime = true;
                return 1;
            }
            Instr {
                opcode: Opcode::RLC,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                self.rlc(r);
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::RRC,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                self.rrc(r);
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::RL,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                self.rl(r);
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::RR,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                self.rr(r);
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::SLA,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                let reg_val = self.rreg8(r);
                let new_val = reg_val << 1;
                self.wreg8(r, new_val);

                self.c_f = (reg_val & 0x80) == 0x80;
                self.n_f = false;
                self.h_f = false;
                self.z_f = new_val == 0;
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::SRA,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                let reg_val = self.rreg8(r);
                let msb = reg_val & 0x80;
                let new_val = (reg_val >> 1) | msb;
                self.wreg8(r, new_val);

                self.c_f = (reg_val & 0x1) == 0x1;
                self.n_f = false;
                self.h_f = false;
                self.z_f = new_val == 0;
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::SWAP,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                let reg_val = self.rreg8(r);
                let upper_nibble = reg_val >> 4;
                let lower_nibble = reg_val & 0xF;
                let new_val = (lower_nibble << 4) | upper_nibble;
                self.wreg8(r, new_val);

                self.z_f = new_val == 0;
                self.n_f = false;
                self.h_f = false;
                self.c_f = false;
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::SRL,
                op1: Some(Operands::R8(r)),
                op2: None,
            } => {
                let reg_val = self.rreg8(r);
                let new_val = reg_val >> 1;
                self.wreg8(r, new_val);

                self.z_f = new_val == 0;
                self.n_f = false;
                self.h_f = false;
                self.c_f = (reg_val & 0x1) == 0x1;
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::BIT,
                op1: Some(Operands::B3(b3)),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);
                let mask = 0x1 << b3;

                self.z_f = (reg_val & mask) == 0;
                self.n_f = false;
                self.h_f = true;
                return if r == HL_PTR { 3 } else { 2 };
            }
            Instr {
                opcode: Opcode::RES,
                op1: Some(Operands::B3(b3)),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);
                let mask = 0x1 << b3;
                let new_val = reg_val & !mask;
                self.wreg8(r, new_val);
                return if r == HL_PTR { 4 } else { 2 };
            }
            Instr {
                opcode: Opcode::SET,
                op1: Some(Operands::B3(b3)),
                op2: Some(Operands::R8(r)),
            } => {
                let reg_val = self.rreg8(r);
                let mask = 0x1 << b3;
                let new_val = reg_val | mask;
                self.wreg8(r, new_val);
                return if r == HL_PTR { 4 } else { 2 };
            }

            _ => unreachable!("Unhandle instruction! {:?}", instr),
        }
    }

    pub fn run_one(&mut self) -> usize {


        //TODO: What happens if we disable IME
        //      then run HALT instruction! Halt bug?
        if self.sleep {
            if let Some(interrupt) = self.bus.query_interrupt() {
                self.sleep = false;
                let cycles = self.handle_interrupt(interrupt);
                self.bus.run_cycles(cycles as u16);
                return cycles;
            } else {
                // If the CPU is halted waiting for next interrupt
                // keep the rest of the system running
                self.bus.run_cycles(1);
                return 1;
            }
        }

        if self.ime {
            if let Some(interrupt) = self.bus.query_interrupt() {
                let cycles = self.handle_interrupt(interrupt);
                self.bus.run_cycles(cycles as u16);
                return cycles
            }
        }

        let next_instr = self.next_instr();
        let cycles = self.execute_instr(next_instr);
        self.bus.run_cycles(cycles as u16);
        cycles
    }

    pub fn handle_interrupt(&mut self, int_source: IntSource) -> usize {
        self.ime = false;
        self.push_stack(self.pc);

        self.pc = match int_source {
            IntSource::VBLANK => 0x40,
            IntSource::LCD => 0x48,
            IntSource::TIMER => 0x50,
            IntSource::SERIAL => 0x58,
            IntSource::JOYPAD => 0x60,
        };

        self.bus.clear_interrupt(int_source);
        return 5;
    }
}
