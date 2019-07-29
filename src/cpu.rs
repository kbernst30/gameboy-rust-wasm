use super::utils;

use wasm_bindgen::prelude::*;
use std::collections::HashMap;

use super::mmu;
use super::game;

// Flag Bits in Register F
const ZERO_BIT: u8 = 7;
const SUBTRACT_BIT: u8 = 6;
const HALF_CARRY_BIT: u8 = 5;
const CARRY_BIT: u8 = 4;

// Timer Constants
const DIVIDER_REGISTER_ADDR: u64 = 0xFF04; // The address of the divier register
const TIMER_ADDR: u64 = 0xFF05; // The timer is located here and counts up a preset interval
const TIMER_MODULATOR_ADDR: u64 = 0xFF06; // The timer modulator that timer resets to on overflow is here

// Timer Controller is 3-bit that controls timer and specifies frequency.
// The 1st 2 bits describe frequency. Here is the mapping:
// 00: 4096 Hz
// 01: 262144 Hz
// 10: 65536 Hz
// 11: 16384 Hz
//
// The third bit specifies if the timer is enabled (1) or disabled (0)
// This is the memory address that the controller is stored at
const TIMER_CONTROLLER_ADDR: u64 = 0xFF07;


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
    mmu: mmu::Mmu,
    registers: HashMap<PairName, Register>,
    program_counter: u16,
    stack_pointer: Register,

    timer_counter: usize,
    divider_counter: usize,
    is_clock_enabled: bool,
}

#[wasm_bindgen]
impl Cpu {
    pub fn new(game: game::Game) -> Cpu {
        utils::set_panic_hook();

        // Initial values are defined in GB architecture

        let mut registers = HashMap::new();
        registers.insert(PairName::AF, Register { value: 0 });
        registers.insert(PairName::BC, Register { value: 0 });
        registers.insert(PairName::DE, Register { value: 0 });
        registers.insert(PairName::HL, Register { value: 0 });

        Cpu {
            mmu: mmu::Mmu::new(game),
            registers,
            program_counter: 0x100,
            stack_pointer: Register { value: 0xFFFE },

            timer_counter: 1024, // Initial value, frequenmcy 4096 (4194304/4096)
            divider_counter: 0,
            is_clock_enabled: true,
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
