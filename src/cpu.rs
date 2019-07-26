use super::utils;

use wasm_bindgen::prelude::*;
use std::collections::HashMap;

// Flag Bits in Register F
const ZERO_BIT: u8 = 7;
const SUBTRACT_BIT: u8 = 6;
const HALF_CARRY_BIT: u8 = 5;
const CARRY_BIT: u8 = 4;

#[derive(Debug, PartialEq, Eq, Hash)]
enum PairName {
    AF,
    BC,
    DE,
    HL,
}

#[derive(Copy, Clone)]
struct RegisterPair {
    low: u8,
    hi: u8,
}

#[repr(C)]
union Register {
    value: u16,
    pair: RegisterPair
}

#[wasm_bindgen]
pub struct Cpu {
    registers: HashMap<PairName, Register>,
    program_counter: u16,
    stack_pointer: Register,
    rom: [u8; 0x10000],
}

#[wasm_bindgen]
impl Cpu {
    pub fn new() -> Cpu {
        utils::set_panic_hook();

        // Initial values are defined in GB architecture

        let mut registers = HashMap::new();
        registers.insert(PairName::AF, Register { value: 0 });
        registers.insert(PairName::BC, Register { value: 0 });
        registers.insert(PairName::DE, Register { value: 0 });
        registers.insert(PairName::HL, Register { value: 0 });

        let mut rom: [u8; 0x10000] = [0; 0x10000];
        rom[0xFF05] = 0x00;
        rom[0xFF06] = 0x00;
        rom[0xFF07] = 0x00;
        rom[0xFF10] = 0x80;
        rom[0xFF11] = 0xBF;
        rom[0xFF12] = 0xF3;
        rom[0xFF14] = 0xBF;
        rom[0xFF16] = 0x3F;
        rom[0xFF17] = 0x00;
        rom[0xFF19] = 0xBF;
        rom[0xFF1A] = 0x7F;
        rom[0xFF1B] = 0xFF;
        rom[0xFF1C] = 0x9F;
        rom[0xFF1E] = 0xBF;
        rom[0xFF20] = 0xFF;
        rom[0xFF21] = 0x00;
        rom[0xFF22] = 0x00;
        rom[0xFF23] = 0xBF;
        rom[0xFF24] = 0x77;
        rom[0xFF25] = 0xF3;
        rom[0xFF26] = 0xF1;
        rom[0xFF40] = 0x91;
        rom[0xFF42] = 0x00;
        rom[0xFF43] = 0x00;
        rom[0xFF45] = 0x00;
        rom[0xFF47] = 0xFC;
        rom[0xFF48] = 0xFF;
        rom[0xFF49] = 0xFF;
        rom[0xFF4A] = 0x00;
        rom[0xFF4B] = 0x00;
        rom[0xFFFF] = 0x00;

        Cpu {
            registers,
            program_counter: 0x100,
            stack_pointer: Register { value: 0xFFFE },
            rom,
        }
    }

    pub fn execute_op(&self) -> usize {
        log!("EXECUTED");
        12
    }

    pub fn test(&self) {
        log!("THIS IS A TEST");
    }
}
