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

// The address of the current scanline
pub const CURRENT_SCANLINE_ADDR: usize = 0xFF44;

// LCD goes through 4 different modes. Bit 1 and 0 (2nd and 1st)
// reflect the current status of the the LCD:
// 00: H-Blank
// 01: V-Blank
// 10: Searching Sprites Atts
// 11: Transferring data to LCD driver
//
// Bits 3, 4, 5 are interrupt enabled flags
// Bit 3: Mode 0 interrupt enabled
// Bit 4: Mode 1 interrupt enabled
// Bit 5: Mode 2 interrupt enabled
//
// Bit 2 is the coincidence flag. It is set to one if
// register 0xFF44 is the same as value as 0xFF45, 0 otherwise
// Bit 6 is an interrupt enabled flag which requests interrupt
// if Bit 2 is 1
//
// The address of the current status of the LCD
pub const LCD_STATUS_ADDR: usize = 0xFF41;

// Bits of the LCD control Register are as follows:
// Bit 7 - LCD Display Enable (0=Off, 1=On)
// Bit 6 - Window Tile Map Display Select (0=9800-9BFF, 1=9C00-9FFF)
// Bit 5 - Window Display Enable (0=Off, 1=On)
// Bit 4 - BG & Window Tile Data Select (0=8800-97FF, 1=8000-8FFF)
// Bit 3 - BG Tile Map Display Select (0=9800-9BFF, 1=9C00-9FFF)
// Bit 2 - OBJ (Sprite) Size (0=8x8, 1=8x16)
// Bit 1 - OBJ (Sprite) Display Enable (0=Off, 1=On)
// Bit 0 - BG Display (for CGB see below) (0=Off, 1=On)
// The address of the LCD control register
pub const LCD_CONTROL_ADDR: usize = 0xFF40;

// The folllowing are addresses which specify where to draw the background and window
// ScrollY (0xFF42): The Y Position of the BACKGROUND where to start drawing the viewing area from
// ScrollX (0xFF43): The X Position of the BACKGROUND to start drawing the viewing area from
// WindowY (0xFF4A): The Y Position of the VIEWING AREA to start drawing the window from
// WindowX (0xFF4B): The X Positions -7 of the VIEWING AREA to start drawing the window from
pub const SCROLL_Y_ADDR: usize = 0xFF42;
pub const SCROLL_X_ADDR: usize = 0xFF43;
pub const WINDOW_Y_ADDR: usize = 0xFF4A;
pub const WINDOW_X_ADDR: usize = 0xFF4B;

// The address of the color pallette
pub const COLOR_PALLETTE_ADDR: usize = 0xFF47;

// The starting address of sprite attribute region
pub const SPRITE_ATTRIBUTE_ADDR: usize = 0xFE00;

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
