mod utils;

extern crate js_sys;

use wasm_bindgen::prelude::*;
use std::collections::HashMap;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

struct Register {
    value: u8,
}

struct Cpu {
    registers: HashMap<&str, Register>,
}

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn console_log(s: &str) {
    log(&s);
}
