use crate::{
    bus::{Bus, Device},
    cart::CartridgeData,
    interrupts::IntSource,
};

#[inline(always)]
fn does_bit3_overflow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;

    return (0xF - a) < b;
}

#[inline(always)]
fn does_bit11_overflow(a: u16, b: u16) -> bool {
    let a = a & 0xFFF;
    let b = b & 0xFFF;

    return (0xFFF - a) < b;
}

#[inline(always)]
fn does_bit3_borrow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;
    return b > a;
}

pub struct Cpu<T: CartridgeData> {
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
    pub bus: Bus<T>,
}

const PAGE0_OFFSET: u16 = 0xFF00;

// Bit indices to address particular register
const A_REG: u8 = 7;
const HL_REG: u8 = 2;

const HL_PTR: u8 = 6;

#[derive(Debug)]
pub enum Reg {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    BC,
    DE,
    HL,
    SP,
}

/*
 * TODO: Replace decode with this to save space?
#[derive(Debug)]
pub enum Instruction2 {
    Nop,                              // 0x00 - No operation
    Load { src: Reg, dest: Reg },      // LD src, dest
    LoadImm8 { reg: Reg, value: u8 },  // LD reg, #8-bit value
    LoadImm16 { reg: Reg, value: u16 },// LD reg, #16-bit value
    LoadMemToReg { reg: Reg, addr: u16 },  // LD (addr), reg
    LoadRegToMem { reg: Reg, addr: u16 },  // LD reg, (addr)
    Add { reg: Reg },                  // ADD reg
    AddImm8 { value: u8 },             // ADD A, #8-bit value
    Sub { reg: Reg },                  // SUB reg
    And { reg: Reg },                  // AND reg
    Or { reg: Reg },                   // OR reg
    Xor { reg: Reg },                  // XOR reg
    Compare { reg: Reg },              // CP reg
    Inc { reg: Reg },                  // INC reg
    Dec { reg: Reg },                  // DEC reg
    Jump { addr: u16 },                // JP addr
    JumpRelative { offset: i8 },       // JR offset
    Call { addr: u16 },                // CALL addr
    Return,                            // RET
    Halt,                              // STOP
    Di,                                // DI (Disable interrupts)
    Ei,                                // EI (Enable interrupts)
    Rst { vector: u16 },               // RST vector
    Rotate { reg: Reg, direction: char }, // RLC, RL, etc.
    BitTest { reg: Reg, bit: u8 },      // BIT bit, reg
    Swap { reg: Reg },                 // SWAP reg
    Shift { reg: Reg, direction: char }, // SLA, SRA, etc.
    Undefined(u8),                     // For undefined opcodes
}
*/

impl<T: CartridgeData> Cpu<T> {

    fn no_op(_cpu: &mut Self, _opcode: u8) -> u8 {
        1
    }

    fn ld_r16_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let imm16 = cpu.load_word();
        cpu.wreg16(r16, imm16);
        3
    }

    fn ld_r16mem_a(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        cpu.wr16mem(r16, cpu.a);
        2
    }

    fn inc_r16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let plus_one = cpu.rreg16(r16).wrapping_add(1);
        cpu.wreg16(r16, plus_one);
        2
    }

    fn inc_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let before = cpu.rreg8(r8);
        cpu.h_f = does_bit3_overflow(before, 1);
        cpu.n_f = false;
        let incre = before.wrapping_add(1);
        cpu.z_f = incre == 0;
        cpu.wreg8(r8, incre);

        return if r8 == HL_PTR { 3 } else { 1 };
    }

    fn dec_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let before = cpu.rreg8(r8);
        let dec = before.wrapping_sub(1);
        cpu.wreg8(r8, dec);

        cpu.z_f = dec == 0;
        cpu.h_f = does_bit3_borrow(before, 1);
        cpu.n_f = true;

        return if r8 == HL_PTR { 3 } else { 1 };
    }

    fn ld_r8_imm8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let imm8 = cpu.load_byte();
        cpu.wreg8(r8, imm8);
        return if r8 == HL_PTR { 3 } else { 2 };
    }

    fn rlca(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rlc(A_REG);
        cpu.z_f = false;
        1
    }

    fn ld_imm16_sp(cpu: &mut Self, _opcode: u8) -> u8 {

        let imm16 = cpu.load_word();

        // This is why: https://rgbds.gbdev.io/docs/v0.8.0/gbz80.7#LD__n16_,SP
        cpu.bus.write(imm16, cpu.sp as u8);
        cpu.bus.write(imm16 + 1, (cpu.sp >> 8) as u8);
        5
    }

    fn add_hl_r16(cpu: &mut Self, opcode: u8) -> u8 {

        let r16 = (opcode >> 4) & 0x3;
        let hl_val = cpu.rreg16(HL_REG);
        let reg_val = cpu.rreg16(r16);
        let (new_val, overflow) = hl_val.overflowing_add(reg_val);
        cpu.wreg16(HL_REG, new_val);
        cpu.n_f = false;
        cpu.h_f = does_bit11_overflow(hl_val, reg_val);
        cpu.c_f = overflow;
        2
    }

    fn ld_a_r16mem(cpu: &mut Self, opcode: u8) -> u8 {
        let r16mem = (opcode >> 4) & 0x3;
        cpu.a = cpu.rr16mem(r16mem);
        2
    }

    fn dec_r16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let minus_one = cpu.rreg16(r16).wrapping_sub(1);
        cpu.wreg16(r16, minus_one);
        2
    }

    fn rrca(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rrc(A_REG);
        cpu.z_f = false;
        1
    }

    fn stop(_cpu: &mut Self, opcode: u8) -> u8 {
        todo!("Stop instruction not implemented! opcode: {}", opcode);
    }

    fn rla(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rl(A_REG);
        cpu.z_f = false;
        1
    }

    fn jr_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let offset = cpu.load_byte() as i8;
        let curr_pc = cpu.pc as i32;
        let new_pc = curr_pc + offset as i32;
        cpu.pc = new_pc as u16;
        return 3;
    }

    fn rra(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rr(A_REG);
        cpu.z_f = false;
        1
    }

    fn jr_cond_imm8(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm8 = cpu.load_byte();
        if cpu.check_cond(cond) {
            let offset = imm8 as i8;
            let curr_pc = cpu.pc as i32;
            let new_pc = curr_pc + offset as i32;

            cpu.pc = new_pc as u16;
            return 3;
        }
        return 2;
    }

    fn daa(cpu: &mut Self, _opcode: u8) -> u8 {
        //https://forums.nesdev.org/viewtopic.php?t=15944
        if cpu.n_f {
            if cpu.c_f {
                cpu.a = cpu.a.wrapping_sub(0x60);
            }
            if cpu.h_f {
                cpu.a = cpu.a.wrapping_sub(0x6);
            }
        } else {
            if cpu.c_f || cpu.a > 0x99 {
                cpu.a = cpu.a.wrapping_add(0x60);
                cpu.c_f = true;
            }
            if cpu.h_f || (cpu.a & 0x0f) > 0x09 {
                cpu.a = cpu.a.wrapping_add(0x6);
            }
        }

        cpu.z_f = cpu.a == 0;
        cpu.h_f = false;
        1
    }

    fn cpl(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.a = !cpu.a;
        cpu.n_f = true;
        cpu.h_f = true;
        1
    }

    fn scf(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = true;
        1
    }

    fn ccf(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = !cpu.c_f;
        return 1;
    }

    fn ld_r8_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let src = opcode & 0x7;
        let dst = (opcode >> 3) & 0x7;
        let d = cpu.rreg8(src);
        cpu.wreg8(dst, d);
        return if (dst == HL_PTR) || (src == HL_PTR) {
            2
        } else {
            1
        };

    }

    fn halt(cpu: &mut Self, _opcode: u8) -> u8 {
        if cpu.ime {
            cpu.sleep = true;
            return 1;
        }

        if !cpu.bus.interrupt_pending() {
            cpu.sleep = true;
            return 1;
        }

        //TODO: Handle HALT bug
        //assert!(false);
        return 1;
    }

    fn add_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);

        cpu.h_f = does_bit3_overflow(reg_val, cpu.a);

        let (new_val, does_overflow) = cpu.a.overflowing_add(reg_val);
        cpu.c_f = does_overflow;
        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.a = new_val;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn adc_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(cpu.a, reg_val);

        let (added, overflow) = reg_val.overflowing_add(cpu.a);

        if cpu.c_f {
            // Next two conditionals check if the carry will overflow
            if does_bit3_overflow(added, 1) {
                cpu.h_f = true;
            }

            cpu.c_f = overflow || added == 0xFF;
            cpu.a = added.wrapping_add(1);
        } else {
            cpu.c_f = overflow;
            cpu.a = added;
        }

        cpu.z_f = cpu.a == 0;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn sub_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x07;
        let reg_val = cpu.rreg8(r8);
        let new_val = cpu.a.wrapping_sub(reg_val);

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);
        cpu.c_f = reg_val > cpu.a;
        cpu.n_f = true;
        cpu.z_f = new_val == 0;

        cpu.a = new_val;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn sbc_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x07;
        let carry = if cpu.c_f { 1 } else { 0 };
        let reg_val = cpu.rreg8(r8);

        cpu.c_f = (reg_val as u16 + carry as u16) > cpu.a as u16;

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);

        cpu.n_f = true;
        cpu.a = cpu.a.wrapping_sub(reg_val);

        if does_bit3_borrow(cpu.a, carry) {
            cpu.h_f = true;
        }

        cpu.a = cpu.a.wrapping_sub(carry);
        cpu.z_f = cpu.a == 0;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn and_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a & cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = true;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn xor_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a ^ cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn or_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a | cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    fn cp_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);
        cpu.c_f = reg_val > cpu.a;
        cpu.n_f = true;
        let new_val = cpu.a.wrapping_sub(reg_val);
        cpu.z_f = new_val == 0;

        return if r8 == HL_PTR { 2 } else { 1 }
    }

    fn add_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.h_f = does_bit3_overflow(imm8, cpu.a);

        let (new_val, does_overflow) = imm8.overflowing_add(cpu.a);
        cpu.c_f = does_overflow;
        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.a = new_val;
        2
    }

    fn adc_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(cpu.a, imm8);

        let (added, overflow) = cpu.a.overflowing_add(imm8);

        if cpu.c_f {
            // Next two conditionals check if the carry will overflow
            if does_bit3_overflow(added, 1) {
                cpu.h_f = true;
            }

            cpu.c_f = overflow || added == 0xFF;
            cpu.a = added.wrapping_add(1);
        } else {
            cpu.c_f = overflow;
            cpu.a = added;
        }

        cpu.z_f = cpu.a == 0;
        return 2;
    }

    fn sub_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let new_val = cpu.a.wrapping_sub(imm8);

        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.c_f = imm8 > cpu.a;
        cpu.n_f = true;
        cpu.z_f = new_val == 0;
        cpu.a = new_val;

        2
    }

    fn sbc_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let carry = if cpu.c_f { 1 } else { 0 };

        cpu.c_f = (imm8 as u16 + carry as u16) > cpu.a as u16;
        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.n_f = true;
        cpu.a = cpu.a.wrapping_sub(imm8);

        if does_bit3_borrow(cpu.a, carry) {
            cpu.h_f = true;
        }

        cpu.a = cpu.a.wrapping_sub(carry);
        cpu.z_f = cpu.a == 0;
        2
    }

    fn and_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a & imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = true;
        cpu.c_f = false;
        2
    }

    fn xor_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a ^ imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        2
    }


    fn or_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a | imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        2
    }

    fn cp_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();

        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.c_f = imm8 > cpu.a;

        cpu.n_f = true;
        let new_val = cpu.a.wrapping_sub(imm8);
        cpu.z_f = new_val == 0;
        2
    }

    fn ret_cond(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        if !cpu.check_cond(cond) {
            return 2;
        }

        cpu.pc = cpu.pop_stack();
        return 5;
    }

    fn pop_r16stk(cpu: &mut Self, opcode: u8) -> u8 {
        let r16stk = (opcode >> 4) & 0x3;
        match r16stk {
            0 => {
                let val = cpu.pop_stack();
                cpu.b = (val >> 8) as u8;
                cpu.c = (val & 0xFF) as u8;
            }
            1 => {
                let val = cpu.pop_stack();
                cpu.d = (val >> 8) as u8;
                cpu.e = (val & 0xFF) as u8;
            }
            2 => {
                let val = cpu.pop_stack();
                cpu.h = (val >> 8) as u8;
                cpu.l = (val & 0xFF) as u8;
            }
            3 => {
                let val = cpu.pop_stack();
                cpu.a = (val >> 8) as u8;
                cpu.z_f = (val & 0x80) == 0x80;
                cpu.n_f = (val & 0x40) == 0x40;
                cpu.h_f = (val & 0x20) == 0x20;
                cpu.c_f = (val & 0x10) == 0x10;
            }
            _ => {
                unreachable!("Invalid pop_r16stk");
            }
        }
        3

    }

    fn push_r16stk(cpu: &mut Self, opcode: u8) -> u8 {

        let r16stk = (opcode >> 4) & 0x3;
        let val = match r16stk {
            0 => ((cpu.b as u16) << 8) | (cpu.c as u16),
            1 => ((cpu.d as u16) << 8) | (cpu.e as u16),
            2 => ((cpu.h as u16) << 8) | (cpu.l as u16),
            3 => {
                let mut val = (cpu.a as u16) << 8;
                val = if cpu.z_f { val | 0x80 } else { val };
                val = if cpu.n_f { val | 0x40 } else { val };
                val = if cpu.h_f { val | 0x20 } else { val };
                val = if cpu.c_f { val | 0x10 } else { val };
                val
            }
            _ => {
                unreachable!("Invalid push_r16stk");
            }
        };

        cpu.push_stack(val);
        return 4;
    }

    fn ret(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.pc = cpu.pop_stack();
        return 4;
    }

    fn reti(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = true;
        cpu.pc = cpu.pop_stack();
        4
    }

    fn jp_cond_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm16 = cpu.load_word();
        if cpu.check_cond(cond) {
            cpu.pc = imm16;
            return 4;
        }

        return 3;
    }

    fn jp_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.pc = imm16;
        4
    }

    fn call_cond_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm16 = cpu.load_word();
        
        if !cpu.check_cond(cond) {
            return 3;
        }

        cpu.push_stack(cpu.pc);
        cpu.pc = imm16;
        return 6;
    }

    fn call_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.push_stack(cpu.pc);
        cpu.pc = imm16;
        6
    }

    fn rst_tgt3(cpu: &mut Self, opcode: u8) -> u8 {
        cpu.push_stack(cpu.pc);
        let tgt = (opcode >> 3) & 0x7;
        cpu.pc = (tgt as u16) << 3;
        return 4;
    }

    fn invalid(_cpu: &mut Self, opcode: u8) -> u8 {
        panic!("Received invalid instruction! opcode: {}", opcode);
    }

    fn ldh_imm8_a(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.bus.write(imm8 as u16 + PAGE0_OFFSET, cpu.a);
        3
    }

    fn ldh_c_a(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.bus.write(PAGE0_OFFSET + cpu.c as u16, cpu.a);
        2
    }

    fn add_sp_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
                let s_i = imm8 as i8;

                let new_sp = cpu.sp.wrapping_add(s_i as u16);
                cpu.z_f = false;
                cpu.n_f = false;
                cpu.h_f = does_bit3_overflow(imm8, cpu.sp as u8);

                let (_, does_bit7_of) = (cpu.sp as u8).overflowing_add(imm8);
                cpu.c_f = does_bit7_of;

                cpu.sp = new_sp;
                4
    }

    fn jp_hl(cpu: &mut Self, _opcode: u8) -> u8 {
                let hl = ((cpu.h as u16) << 8) | cpu.l as u16;
                cpu.pc = hl;
                1
    }

    fn ld_imm16_a(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.bus.write(imm16, cpu.a);
        4
    }

    fn ldh_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.bus.read(imm8 as u16 + PAGE0_OFFSET);
        3
    }

    fn ldh_a_c(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.a = cpu.bus.read(cpu.c as u16 + PAGE0_OFFSET);
        2
    }

    fn prefix(cpu: &mut Self, _opcode: u8) -> u8 {

        let next_byte = cpu.load_byte();
        let cycles = match next_byte {
            0..=0x7 => { Self::prefix_rlc(cpu, next_byte) },
            0x8..=0xF => { Self::prefix_rrc(cpu, next_byte) },
            0x10..=0x17 => { Self::prefix_rl(cpu, next_byte) },
            0x18..=0x1F => { Self::prefix_rr(cpu, next_byte) },
            0x20..=0x27 => { Self::prefix_sla(cpu, next_byte) },
            0x28..=0x2f => { Self::prefix_sra(cpu, next_byte) },
            0x30..=0x37 => { Self::prefix_swap(cpu, next_byte) },
            0x38..=0x3F => { Self::prefix_srl(cpu, next_byte) },
            0x40..=0x7F => { Self::prefix_bit(cpu, next_byte) },
            0x80..=0xBF => { Self::prefix_res(cpu, next_byte) },
            0xC0..=0xFF => { Self::prefix_set(cpu, next_byte) },
        };

        cycles

            /*
        let next_byte = cpu.load_byte();
        let instr = Self::cb_prefix_decode(next_byte);
        let cycles = cpu.execute_instr(instr);

        cycles as u8
            */
    }

    fn di(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = false;
        1
    }

    fn ei(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = true;
        1
    }
    
    fn ld_hl_sp_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
                let sp = cpu.sp as i32;
                let i_8 = imm8 as i8;
                let new_hl = (sp + i_8 as i32) as u16;

                cpu.z_f = false;
                cpu.n_f = false;
                cpu.h_f = does_bit3_overflow(imm8 as u8, cpu.sp as u8);

                // Checking for signed byte add overflow
                let (_, does_of) = (cpu.sp as u8).overflowing_add(imm8);

                cpu.c_f = does_of;
                cpu.h = (new_hl >> 8) as u8;
                cpu.l = (new_hl & 0xFF) as u8;
                3
    }
    fn ld_sp_hl(cpu: &mut Self, _opcode: u8) -> u8 {
        let new_sp = ((cpu.h as u16) << 8) | cpu.l as u16;
        cpu.sp = new_sp;
        2
    }

    fn ld_a_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.a = cpu.bus.read(imm16);
        4
    }
    
    fn prefix_rlc(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rlc(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_rrc(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rrc(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_rl(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rl(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_rr(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rr(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_sla(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let new_val = reg_val << 1;
        cpu.wreg8(r, new_val);

        cpu.c_f = (reg_val & 0x80) == 0x80;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.z_f = new_val == 0;
        return if r == HL_PTR { 4 } else { 2 };

    }

    fn prefix_sra(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let msb = reg_val & 0x80;
        let new_val = (reg_val >> 1) | msb;
        cpu.wreg8(r, new_val);

        cpu.c_f = (reg_val & 0x1) == 0x1;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.z_f = new_val == 0;
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_swap(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let upper_nibble = reg_val >> 4;
        let lower_nibble = reg_val & 0xF;
        let new_val = (lower_nibble << 4) | upper_nibble;
        cpu.wreg8(r, new_val);

        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_srl(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let new_val = reg_val >> 1;
        cpu.wreg8(r, new_val);

        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = (reg_val & 0x1) == 0x1;
        return if r == HL_PTR { 4 } else { 2 };
    }

    fn prefix_bit(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;

        cpu.z_f = (reg_val & mask) == 0;
        cpu.n_f = false;
        cpu.h_f = true;
        return if r == HL_PTR { 3 } else { 2 };
    }

    fn prefix_res(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;
        let new_val = reg_val & !mask;
        cpu.wreg8(r, new_val);
        return if r == HL_PTR { 4 } else { 2 };

    }

    fn prefix_set(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;
        let new_val = reg_val | mask;
        cpu.wreg8(r, new_val);
        return if r == HL_PTR { 4 } else { 2 };
    }

    const INSTR_HANDLERS : [fn(&mut Self, u8) -> u8; 256] = [
        Self::no_op, // 0x00
        Self::ld_r16_imm16, // 0x01
        Self::ld_r16mem_a, // 0x02
        Self::inc_r16, //0x03
        Self::inc_r8, //0x04
        Self::dec_r8, //0x05
        Self::ld_r8_imm8, //0x06
        Self::rlca, //0x07
        Self::ld_imm16_sp, //0x08
        Self::add_hl_r16, //0x09
        Self::ld_a_r16mem, //0x0A
        Self::dec_r16, //0x0B
        Self::inc_r8, //0x0C
        Self::dec_r8, //0x0D
        Self::ld_r8_imm8, //0x0E
        Self::rrca, //0x0F
        Self::stop, //0x10
        Self::ld_r16_imm16, //0x11
        Self::ld_r16mem_a, //0x12
        Self::inc_r16, //0x13
        Self::inc_r8, //0x14
        Self::dec_r8, //0x15
        Self::ld_r8_imm8, //0x16
        Self::rla, //0x17
        Self::jr_imm8, //0x18
        Self::add_hl_r16, //0x19
        Self::ld_a_r16mem, //0x1A
        Self::dec_r16, //0x1B
        Self::inc_r8, //0x1C
        Self::dec_r8, //0x1D
        Self::ld_r8_imm8, //0x1E
        Self::rra, //0x1F
        Self::jr_cond_imm8, //0x20
        Self::ld_r16_imm16, //0x21
        Self::ld_r16mem_a, //0x22
        Self::inc_r16, //0x23
        Self::inc_r8, //0x24
        Self::dec_r8, //0x25
        Self::ld_r8_imm8, //0x26
        Self::daa, //0x27
        Self::jr_cond_imm8, //0x28
        Self::add_hl_r16, //0x29
        Self::ld_a_r16mem, //0x2A
        Self::dec_r16, //0x2B
        Self::inc_r8, //0x2C
        Self::dec_r8, //0x2D
        Self::ld_r8_imm8, //0x2E
        Self::cpl, //0x2F
        Self::jr_cond_imm8, //0x30
        Self::ld_r16_imm16, //0x31
        Self::ld_r16mem_a, //0x32
        Self::inc_r16, //0x33
        Self::inc_r8, //0x34
        Self::dec_r8, //0x35
        Self::ld_r8_imm8, //0x36
        Self::scf, //0x37
        Self::jr_cond_imm8, //0x38
        Self::add_hl_r16, //0x39
        Self::ld_a_r16mem, //0x3A
        Self::dec_r16, //0x3B
        Self::inc_r8, //0x3C
        Self::dec_r8, //0x3D
        Self::ld_r8_imm8, //0x3E
        Self::ccf, //0x3F
        Self::ld_r8_r8, //0x40 ...
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::halt,  //0x76
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, 
        Self::ld_r8_r8, //0x7F
        Self::add_a_r8, //0x80
        Self::add_a_r8, //0x81
        Self::add_a_r8, //0x82
        Self::add_a_r8, //0x83
        Self::add_a_r8, //0x84
        Self::add_a_r8, //0x85
        Self::add_a_r8, //0x86
        Self::add_a_r8, //0x87
        Self::adc_a_r8, //0x88
        Self::adc_a_r8, //0x89
        Self::adc_a_r8, //0x8A
        Self::adc_a_r8, //0x8B
        Self::adc_a_r8, //0x8C
        Self::adc_a_r8, //0x8D
        Self::adc_a_r8, //0x8E
        Self::adc_a_r8, //0x8F
        Self::sub_a_r8, //0x90
        Self::sub_a_r8, //0x91
        Self::sub_a_r8, //0x92
        Self::sub_a_r8, //0x93
        Self::sub_a_r8, //0x94
        Self::sub_a_r8, //0x95
        Self::sub_a_r8, //0x96
        Self::sub_a_r8, //0x97
        Self::sbc_a_r8, //0x98
        Self::sbc_a_r8, //0x99
        Self::sbc_a_r8, //0x9A
        Self::sbc_a_r8, //0x9B
        Self::sbc_a_r8, //0x9C
        Self::sbc_a_r8, //0x9D
        Self::sbc_a_r8, //0x9E
        Self::sbc_a_r8, //0x9F
        Self::and_a_r8, //0xA0
        Self::and_a_r8, //0xA1
        Self::and_a_r8, //0xA2
        Self::and_a_r8, //0xA3
        Self::and_a_r8, //0xA4
        Self::and_a_r8, //0xA5
        Self::and_a_r8, //0xA6
        Self::and_a_r8, //0xA7
        Self::xor_a_r8, //0xA8
        Self::xor_a_r8, //0xA9
        Self::xor_a_r8, //0xAA
        Self::xor_a_r8, //0xAB
        Self::xor_a_r8, //0xAC
        Self::xor_a_r8, //0xAD
        Self::xor_a_r8, //0xAE
        Self::xor_a_r8, //0xAF
        Self::or_a_r8, //0xB0
        Self::or_a_r8, //0xB1
        Self::or_a_r8, //0xB2
        Self::or_a_r8, //0xB3
        Self::or_a_r8, //0xB4
        Self::or_a_r8, //0xB5
        Self::or_a_r8, //0xB6
        Self::or_a_r8, //0xB7
        Self::cp_a_r8, //0xB8
        Self::cp_a_r8, //0xB9
        Self::cp_a_r8, //0xBA
        Self::cp_a_r8, //0xBB
        Self::cp_a_r8, //0xBC
        Self::cp_a_r8, //0xBD
        Self::cp_a_r8, //0xBE
        Self::cp_a_r8, //0xBF
        Self::ret_cond, //0xC0
        Self::pop_r16stk, //0xC1
        Self::jp_cond_imm16, //0xC2
        Self::jp_imm16, //0xC3
        Self::call_cond_imm16, //0xC4
        Self::push_r16stk, //0xC5
        Self::add_a_imm8, //0xC6
        Self::rst_tgt3, //0xC7
        Self::ret_cond, //0xC8
        Self::ret, //0xC9
        Self::jp_cond_imm16, //0xCA
        Self::prefix, //0xCB prefix!
        Self::call_cond_imm16, //0xCC
        Self::call_imm16, //0xCD
        Self::adc_a_imm8, //0xCE
        Self::rst_tgt3, //0xCF
        Self::ret_cond, //0xD0
        Self::pop_r16stk, //0xD1
        Self::jp_cond_imm16, //0xD2
        Self::invalid, //0xD3
        Self::call_cond_imm16, //0xD4
        Self::push_r16stk, //0xD5
        Self::sub_a_imm8, //0xD6
        Self::rst_tgt3, //0xD7
        Self::ret_cond, //0xD8
        Self::reti, //0xD9
        Self::jp_cond_imm16, //0xDA
        Self::invalid, //0xDB
        Self::call_cond_imm16, //0xDC
        Self::invalid, //0xDD
        Self::sbc_a_imm8, //0xDE
        Self::rst_tgt3, //0xDF
        Self::ldh_imm8_a, //0xE0
        Self::pop_r16stk, //0xE1
        Self::ldh_c_a, //0xE2
        Self::invalid, //0xE3
        Self::invalid, //0xE4
        Self::push_r16stk, //0xE5
        Self::and_a_imm8, //0xE6
        Self::rst_tgt3, //0xE7
        Self::add_sp_imm8, //0xE8
        Self::jp_hl, //0xE9
        Self::ld_imm16_a, //0xEA
        Self::invalid, //0xEB
        Self::invalid, //0xEC
        Self::invalid, //0xED
        Self::xor_a_imm8, //0xEE
        Self::rst_tgt3, //0xEF
        Self::ldh_a_imm8, //0xF0
        Self::pop_r16stk, //0xF1
        Self::ldh_a_c, //0xF2
        Self::di, //0xF3
        Self::invalid, //0xF4
        Self::push_r16stk, //0xF5
        Self::or_a_imm8, //0xF6
        Self::rst_tgt3, //0xF7
        Self::ld_hl_sp_imm8, //0xF8
        Self::ld_sp_hl, //0xF9
        Self::ld_a_imm16, //0xFA
        Self::ei, //0xFB
        Self::invalid, //0xFC
        Self::invalid, //0xFD
        Self::cp_a_imm8, //0xFE
        Self::rst_tgt3, //0xFF
    ];

    //TODO: Add an API to build the CPU in a state that
    //      does not skip the boot rom
    pub fn new(bus: Bus<T>) -> Self {
        Cpu {
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
        }

        // I don't remember exactly why this was
        // here, but I think this might be here
        // back when the PPU was just a bank of memory
        // and we needed 0xFF44 to always return 0x90
        // to make it work with the Gameboy Doctor tests.
        // TODO: Make sure this isn't needed anymore and remove
        // this dead code.  We shouldn't need GB doctor anymore.
        //
        // Temporary to make LCD work with test ROMs
        // cpu.bus.write(0xFF44, 0x90);
    }

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
    fn check_cond(&self, cond: u8) -> bool {
        match cond {
            0 => return !self.z_f,
            1 => return self.z_f,
            2 => return !self.c_f,
            3 => return self.c_f,
            _ => unreachable!("invalid check_cond"),
        }
    }

    #[inline(always)]
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
            _ => unreachable!("wr16mem with invalid bit index! {r16mem}"),
        }
    }

    #[inline(always)]
    fn push_stack(&mut self, val: u16) {
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val >> 8) as u8);
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val & 0xFF) as u8);
    }

    #[inline(always)]
    fn pop_stack(&mut self) -> u16 {
        let mut ret = self.bus.read(self.sp) as u16;
        self.sp = self.sp + 1;
        ret |= (self.bus.read(self.sp) as u16) << 8;
        self.sp = self.sp + 1;
        return ret;
    }

    #[inline(always)]
    fn load_byte(&mut self) -> u8 {
        let next_byte = self.bus.read(self.pc);
        self.pc += 1;
        return next_byte;
    }

    #[inline(always)]
    fn load_word(&mut self) -> u16 {
        let next = self.load_byte();
        let next_next = self.load_byte();

        return (next as u16) | ((next_next as u16) << 8);
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

    pub fn is_passed(&self) -> bool {
        return self.bus.is_passed();
    }

    pub fn run_one(&mut self) -> usize {
        // Review this and make sure all four conditions are handled correctly
        // with IME and HALT
        if self.sleep {
            if self.bus.interrupt_pending() {
                self.sleep = false;
            }

            self.bus.run_cycles(1);
            return 1;
        }

        if self.ime {
            if let Some(interrupt) = self.bus.query_interrupt() {
                let cycles = self.handle_interrupt(interrupt);
                self.bus.run_cycles(cycles as u16);
                return cycles;
            }
        }


        unsafe { core::arch::asm!("NOP"); }
        unsafe { core::arch::asm!("NOP"); }
        unsafe { core::arch::asm!("NOP"); }
        let opcode = self.bus.read(self.pc);
        self.pc += 1;
        let cycles = Self::INSTR_HANDLERS[opcode as usize](self, opcode).into();
        unsafe { core::arch::asm!("NOP"); }
        unsafe { core::arch::asm!("NOP"); }
        unsafe { core::arch::asm!("NOP"); }

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
