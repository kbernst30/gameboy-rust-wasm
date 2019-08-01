#[macro_use]
mod utils;

mod cpu;
mod game;
mod mmu;
mod ops;

extern crate js_sys;
extern crate web_sys;

use wasm_bindgen::prelude::*;

use std::collections::HashMap;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct Emulator {
    cpu: cpu::Cpu,
}

#[wasm_bindgen]
impl Emulator {
    pub fn new(game: game::Game) -> Emulator {
        Emulator {
            cpu: cpu::Cpu::new(game),
        }
    }

    pub fn update(&mut self) {
        // Gameboy can execute 4194304 cycles per second and
        // we will be emulating at 60 fps. In other words, this
        // function should be called 60 times per second as it represents
        // a single frame update

        // 4194304/60 = 66905
        let max_cycles_per_frame = 69905;
        let mut cycles_this_update = 0;

        while cycles_this_update < max_cycles_per_frame {
            let cycles = self.cpu.execute_op();
            cycles_this_update += cycles;

            self.cpu.update_timers(&cycles);
            self.cpu.update_graphics(&cycles);
            self.cpu.do_interrupts();
        }

        // Frame Update
    }
}
