use super::utils;

use wasm_bindgen::prelude::*;
use std::collections::HashMap;

use super::mmu;
use super::game;

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

pub struct Cpu {
    mmu: mmu::Mmu,
    registers: HashMap<PairName, Register>,
    program_counter: u16,
    stack_pointer: Register,
    divider_counter: u16,
    interrupt_master: bool,
}

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
            divider_counter: 0,
            interrupt_master: true,
        }
    }

    pub fn execute_op(&self) -> usize {
        log!("EXECUTED");
        12
    }

    pub fn update_timers(&mut self, cycles: &usize) {
        // The Divider Register counts up continuously from 0 to 255
		// Overflow causes it to reset to 0
		// It can't be paused by isClockEnabled and counts up at frequency of 16382 hz
		// which is every 256 clock cycles
        self.do_divider_register(&(*cycles as u16));

        // If clock is enabled, do updates
        if self.is_clock_enabled() {
            // Update based on how many cycles passed
			// The timer increments when this hits 0 as that is based on the
			// frequency in which the timer should increment
            self.mmu.decrease_timer_counter(cycles);

            if *self.mmu.get_timer_counter() <= 0 {
                // We need to reset the counter value so timer can increment again at the
				// correct frequenct
				self.mmu.set_clock_frequency();

                // Need to account for overflow - if overflow then we can write	the value
				// that is held in the modulator addr and request Timer Interrupt which is
				// bit 2 of the interrupt register in memory
				// Otherwise we can just increment the timer
                if self.mmu.read_memory(&utils::TIMER_ADDR) == 255 {
                    self.mmu.write_memory(&utils::TIMER_ADDR, self.mmu.read_memory(&utils::TIMER_MODULATOR_ADDR));
                    self.request_interrupt(2);
                } else {
                    self.mmu.write_memory(&utils::TIMER_ADDR, self.mmu.read_memory(&utils::TIMER_ADDR) + 1);
                }
            }
        }
    }

    pub fn do_interrupts(&mut self) {
        if self.interrupt_master {
            let interrupt_request_value = self.mmu.read_memory(&utils::INTERRUPT_REQUEST_ADDR);
            let interrupt_enabled_value = self.mmu.read_memory(&utils::INTERRUPT_ENABLED_ADDR);

            // If any interrupts have been requested (i.e. any bits are set)
            if interrupt_request_value > 0 {

                // Go through all interrupt bits that might be set - handles priority
                for i in 0..5 {
                    // If interrupt is requested
                    if self.check_interrupt_bit(&i, &interrupt_request_value) {
                        // If interrupt is enabled
                        if self.check_interrupt_bit(&i, &interrupt_enabled_value) {
                            // Service interrupt
                            self.service_interrupt(&i);
                        }
                    }
                }
            }
        }
    }

    fn push_word_to_stack(&mut self, word: &u16) {
        let hi: u8 = word.checked_shr(8).unwrap_or(0) as u8;
        let lo = (word & 0xFF) as u8;

        unsafe {
            self.stack_pointer.value -= 1;
            self.mmu.write_memory(&(self.stack_pointer.value as usize), hi);
            self.stack_pointer.value -= 1;
            self.mmu.write_memory(&(self.stack_pointer.value as usize), lo);
        }
    }

    fn pop_word_from_stack(&mut self) -> u16 {
        unsafe {
            let stack_pointer = self.stack_pointer.value as usize;
            let mut word = (self.mmu.read_memory(&(stack_pointer + 1)) as u16) << 8;
            word |= self.mmu.read_memory(&stack_pointer) as u16;
            self.stack_pointer.value += 2;
            word
        }

    }

    fn do_divider_register(&mut self, cycles: &u16) {
        self.divider_counter += cycles;
        if self.divider_counter >= 255 {
            self.divider_counter = 0;
            self.mmu.increment_divider_register();
        }
    }

    fn is_clock_enabled(&self) -> bool {
        let timer_controller_value = self.mmu.read_memory(&utils::TIMER_CONTROLLER_ADDR);

        // 8 = 0b100 -> Test the third bit (if clock is enabled) with a bit wise AND
        timer_controller_value & 8 > 0
    }

    fn request_interrupt(&mut self, bit: u8) {
        // bit = 0: V-Blank Interrupt
		// bit = 1: LCD Interrupt
		// bit = 2: Timer Interrupt
		// bit = 4: Joypad Interrupt

        let mut interrupt_request_value = self.mmu.read_memory(&utils::INTERRUPT_REQUEST_ADDR);
        interrupt_request_value |= match bit {
            0 => 1,  // 0b00001
            1 => 2,  // 0b00010
            2 => 4,  // 0b00100
            4 => 16, // 0b10000
            _ => interrupt_request_value // Do nothing
        };

        self.mmu.write_memory(&utils::INTERRUPT_REQUEST_ADDR, interrupt_request_value);
    }

    fn check_interrupt_bit(&self, bit: &u8, interrupt_register_value: &u8) -> bool {
        match bit {
            0 => 1 & interrupt_register_value > 0,
            1 => 2 & interrupt_register_value > 0,
            2 => 4 & interrupt_register_value > 0,
            4 => 16 & interrupt_register_value > 0,
            _ => false
        }
    }

    fn service_interrupt(&mut self, bit: &u8) {
        // The requested interrupt bit is performed
		// Interrupt operations are found in the following locations in game memory
		// V-Blank: 0x40
		// LCD: 0x48
		// TIMER: 0x50
		// JOYPAD: 0x60

        // We need to flip the master interrupt switch off and then turn off the
		// bit in the interrupt request register for the interrupt we are running
		self.interrupt_master = false;
		let mut interrupt_request_value = self.mmu.read_memory(&utils::INTERRUPT_REQUEST_ADDR);

        // XOR will turn off the bits because we know it is set in the register
		// It will leave the other ones intact as they are XOR-ing with 0
        interrupt_request_value ^= match bit {
            0 => 1,  // 0b00001
            1 => 2,  // 0b00010
            2 => 4,  // 0b00100
            4 => 16, // 0b10000
            _ => interrupt_request_value // Do nothing
        };

        self.mmu.write_memory(&utils::INTERRUPT_REQUEST_ADDR, interrupt_request_value);

        // Save current execution address by pushing onto the stack
        let current_pc = self.program_counter.clone();
        self.push_word_to_stack(&current_pc);

        // set the PC to the address of the requested interrupt
        self.program_counter = match bit {
            0 => 0x40,  // 0b00001
            1 => 0x48,  // 0b00010
            2 => 0x50,  // 0b00100
            4 => 0x60, // 0b10000
            _ => self.program_counter // Do nothing
        };
    }
}
