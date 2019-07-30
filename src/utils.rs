extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

// Flag Bits in Register F
pub const ZERO_BIT: u8 = 7;
pub const SUBTRACT_BIT: u8 = 6;
pub const HALF_CARRY_BIT: u8 = 5;
pub const CARRY_BIT: u8 = 4;

// Timer Constants
pub const DIVIDER_REGISTER_ADDR: usize = 0xFF04; // The address of the divier register
pub const TIMER_ADDR: usize = 0xFF05; // The timer is located here and counts up a preset interval
pub const TIMER_MODULATOR_ADDR: usize = 0xFF06; // The timer modulator that timer resets to on overflow is here

// Timer Controller is 3-bit that controls timer and specifies frequency.
// The 1st 2 bits describe frequency. Here is the mapping:
// 00: 4096 Hz
// 01: 262144 Hz
// 10: 65536 Hz
// 11: 16384 Hz
//
// The third bit specifies if the timer is enabled (1) or disabled (0)
// This is the memory address that the controller is stored at
pub const TIMER_CONTROLLER_ADDR: usize = 0xFF07;

// There are 4 types of interrupts that can occur and the following are the bits
// that are set in the enabled register and request register when they occur
// Note: the lower the bit, the higher priority of the interrupt
// Bit 0: V-Blank Interupt
// Bit 1: LCD Interupt
// Bit 2: Timer Interupt
// Bit 4: Joypad Interupt
//
// Interrupt Register Address Constants
pub const INTERRUPT_ENABLED_ADDR: usize = 0xFFFF;
pub const INTERRUPT_REQUEST_ADDR: usize = 0xFF0F;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
