use super::utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Game {
    memory_bank_one:   [u8; 0x8000],
    memory_bank_two:   [u8; 0x8000],
    memory_bank_three: [u8; 0x8000],
    memory_bank_four:  [u8; 0x8000],
}

#[wasm_bindgen]
impl Game {
    pub fn new() -> Game {
        Game {
            memory_bank_one:   [0; 0x8000],
            memory_bank_two:   [0; 0x8000],
            memory_bank_three: [0; 0x8000],
            memory_bank_four:  [0; 0x8000],
        }
    }

    pub fn load_game_memory() {
        log!("Loading game");
    }

    pub fn read_catridge_data(&self, address: usize) -> u8 {
        return 0;
    }
}
