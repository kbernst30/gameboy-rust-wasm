use super::utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Game {
    memory: [u8; 0x200000]
}

impl Game {
    pub fn new() -> Game {
        Game {
            memory: [0; 0x200000]
        }
    }

    pub fn load_game_memory() {
        log!("Loading game");
    }
}
