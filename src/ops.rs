use super::utils;
use super::cpu;

use wasm_bindgen::prelude::*;

pub fn do_execute_op(mut cpu: &cpu::Cpu, operation: u8) -> usize {
    match operation {
        // NOP
        0x00 => 4,

        // 16 Bit Loads
        0x01 => cpu_16_bit_load(cpu, &cpu::PairName::BC),
        0x11 => cpu_16_bit_load(cpu, &cpu::PairName::BC),
        0x21 => cpu_16_bit_load(cpu, &cpu::PairName::BC),
        0x31 => cpu_16_bit_load(cpu, &cpu::PairName::BC),

        _    => 4
    }
}

fn cpu_16_bit_load(mut cpu: &cpu::Cpu, pair: &cpu::PairName) -> usize {
    unsafe {
        let first_address = (cpu.program_counter + 1) as usize;
        let second_address = (cpu.program_counter + 1) as usize;

        let mut data: u16 = (cpu.mmu.read_memory(&first_address) as u16) << 8;
        data |= cpu.mmu.read_memory(&second_address) as u16;

        match cpu.registers.get(pair) {
            Some(register) => register.value = data,
            Non => log!("No register found")
        };
    }

    12
}

// fn cpu_8_bit_load