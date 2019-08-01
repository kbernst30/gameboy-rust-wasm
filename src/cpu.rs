use super::utils;

use wasm_bindgen::prelude::*;
use std::collections::HashMap;

use super::mmu;
use super::game;
use super::ops;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PairName {
    AF,
    BC,
    DE,
    HL,
}

#[derive(Copy, Clone)]
pub struct RegisterPair {
    pub low: u8,
    pub hi: u8,
}

#[repr(C)]
pub union Register {
    pub value: u16,
    pub pair: RegisterPair
}

pub struct Cpu {
    pub mmu: mmu::Mmu,
    pub registers: HashMap<PairName, Register>,
    pub program_counter: u16,
    stack_pointer: Register,
    divider_counter: u16,
    interrupt_master: bool,
    scanline_counter: u16,
    screen_data: Vec<u8>,
    halted: bool,
}

impl Cpu {
    pub fn new(game: game::Game) -> Cpu {
        utils::set_panic_hook();

        // Initial values are defined ißn GB architecture

        let mut registers = HashMap::new();
        registers.insert(PairName::AF, Register { value: 0 });
        registers.insert(PairName::BC, Register { value: 0 });
        registers.insert(PairName::DE, Register { value: 0 });
        registers.insert(PairName::HL, Register { value: 0 });

        // A flat vec, needs to be width (160) * height (144) * 3 (RGB)
        let screen_data = (0..160 * 144 * 3)
            .map(|i| { 0 })
            .collect();

        Cpu {
            mmu: mmu::Mmu::new(game),
            registers,
            program_counter: 0x100,
            stack_pointer: Register { value: 0xFFFE },
            divider_counter: 0,
            interrupt_master: true,
            scanline_counter: 456,
            // screen_data: [[[0; 160]; 144]; 3],
            screen_data,
            halted: false,
        }
    }

    pub fn execute_op(&mut self) -> usize {
        let mut cycles: usize;

        if !self.halted {
            let next_op = self.mmu.read_memory(&(self.program_counter as usize));
            cycles = ops::do_execute_op(self, next_op);
            self.program_counter += 1;
        } else {
            cycles = 4;
        }

        // TODO some stuff with interrupts

        cycles
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

    pub fn update_graphics(&mut self, cycles: &usize) {
        // Deal with setting LCD status
        self.set_lcd_status();

        // If LCD Display is enabled, decerement counter by number of cycles
		// Otherwise do nothing
        if self.is_lcd_enabled() {
            self.scanline_counter -= *cycles as u16;
        } else {
            return;
        }

        // If scanline counter hit 0, we need to move onto the next scanline
		// Current scanline is found in memory in 0xFF44
		// We can't write to this memory location using write functionas doing so
		// should cause the value here to be set to 0 so access the memory directly
		// Scanline 0 - 143 (144 in total) need to be rendered onto the screen
		// Scanline 144 - 153 is the Vertical Blank Period and we need to
		// request the Vertical Blank Interrupt
		// If Scanline is greater than 153, reset to 0
        if self.scanline_counter <= 0 {
            // Move onto next scanline
            self.mmu.increment_scanline_value();
            let current_line = self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR);

            self.scanline_counter = 456;

            // Are we in vertical blank period?
            if current_line == 144 {
                self.request_interrupt(0);
            } else if current_line > 153 {
                // Reset if passed scanline 153 (max scanline)
                self.mmu.reset_scanline_value();
            } else {
                // any visible scanline should be drawn
                self.draw_scanline();
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

    fn set_lcd_status(&mut self) {
        // LCD status is stored in memory address 0xFF41
		// The first 2 bits represent the mode of the LCD and are as follows:
		// 00 (0): Horizontal-Blank
		// 01 (1): Vertical-Blank
		// 10 (2): Searching Sprites Atts
		// 11 (3): Transfering Data to LCD Driver

        let mut lcd_status = self.mmu.read_memory(&utils::LCD_STATUS_ADDR);
        if !self.is_lcd_enabled() {
            // If LCD is disabled, set LCD mode to 1 and reset scanline
            self.scanline_counter = 456;
            self.mmu.reset_scanline_value();
            lcd_status &= 252; // 252 = 0b11111100
            lcd_status |= 1; // Set Bit 0 to ensure proper mode is equal to 1
            self.mmu.write_memory(&utils::LCD_STATUS_ADDR, lcd_status);
            return;
        }

        // Each scanline takes 456 clock cycles and this is further split up
		// If within the first 80 cycles of the 456, we should be in mode 2
		// If within the next 172 cycles of the 456, we should be in  mode 3
		// Past this point up to the end of the 456, we should be in mode 0
		// If within V-Blank (scanline 144 - 153) we should be in mode 1

        let current_scanline = self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR);
        let current_mode = lcd_status & 0x3;

        let mut mode: u8 = 0;
        let mut requested_interrupt = false;

        if current_scanline >= 144 {
            // If in V-Blank (recall drawing line greater than or equal to 144)
            // In this case we need to set the mode to 1
            lcd_status |= 1; // Set bit 0 to 1
            lcd_status &= 253; // 253 = 0b11111101 - Unsets bit 1
            requested_interrupt = lcd_status & 16 > 0; // 16 = 0b00010000 - Tests bit 4 for interrupt enabled

        } else {
            let mode_2_bounds = 458 - 80;
            let mode_3_bounds = mode_2_bounds - 172;

            if self.scanline_counter >= mode_2_bounds {
                // mode 2
                mode = 2;
                lcd_status &= 254; // 254 = 0b11111110 - Set bit 0 to 0
                lcd_status |= 2; // 2 = 0b00000010 - Sets bit 1 to 1
                requested_interrupt = lcd_status & 32 > 0; // 32 = 0b00100000 - Tests bit 5 for interrupt enabled

            } else if self.scanline_counter >= mode_3_bounds {
                // mode 3
                mode = 3;
                lcd_status |= 3; // 3 = 0b00000011 - Sets bit 1 and 0 to 1

            } else {
                // mode 0
                mode = 0;
                lcd_status &= 252; // 252 = 0b11111100 - Set bit 1 and 0 to 0
                requested_interrupt = lcd_status & 8 > 0; // 8 = 0b00001000 - Tests bit 3 for interrupt enabled
            }
        }

        // Mode has changed and we wanted an interrupt, so request it
        if requested_interrupt && mode != current_mode {
            // 1 is for LCD interrupt
            self.request_interrupt(1);
        }

        // Check coincidence flag
        // Bit 2 of Status register is Coincedence Flag
		// This should be set to true if current scanline (0xFF44) is equal to
		// value in  register 0xFF45. Otherwise turn it off.
		// If bit 6 is set in the Status register and the coincedence flag is turned
		// on, then request an LCD Interrupt
        if current_scanline == self.mmu.read_memory(&0xFF45) {
            lcd_status |= 4; // 4 = 0b00000100 - Sets bit 2 to 1
            if lcd_status & 64 > 0 {
                // 64 = 0b01000000 - Checks bit 6, if set, then request LCD interrupt
                self.request_interrupt(1);
            }

        } else {
            lcd_status &= 251; // 251 = 0b11111011 - Reset bit 2 to 0
        }

        // Ensure LCD status is properly written to memory
        self.mmu.write_memory(&utils::LCD_STATUS_ADDR, lcd_status);
    }

    fn is_lcd_enabled(&self) -> bool {
        // Bit 7 of LCD control register specifies if LCD is enabled or not
        let lcd_control = self.mmu.read_memory(&utils::LCD_CONTROL_ADDR);
        lcd_control & 128 > 0 // 128 = 0b10000000
    }

    fn draw_scanline(&mut self) {
        let lcd_control = self.mmu.read_memory(&utils::LCD_CONTROL_ADDR);

        // If bit 0 is set, than the background display is enabled and we should draw
        if lcd_control & 1 > 0 {
            self.render_tiles(&lcd_control);
        }

        // If bit 1 is set, tham the sprite display is enabled and we should draw
        if lcd_control & 2 > 0 {
            self.render_sprites(&lcd_control);
        }
    }

    fn render_tiles(&mut self, lcd_control: &u8) {
        let mut tile_data: u16 = 0;
        let mut background_memory: u16 = 0;
        let mut unsigned = true;

        // Determine where to draw the visual background and the window
        let scroll_y = self.mmu.read_memory(&utils::SCROLL_Y_ADDR);
        let scroll_x = self.mmu.read_memory(&utils::SCROLL_X_ADDR);
        let window_y = self.mmu.read_memory(&utils::WINDOW_Y_ADDR);
        let window_x = self.mmu.read_memory(&utils::WINDOW_X_ADDR) - 7;

        let mut using_window = false;

        // Bit 5 of LCD control register determines if the window is enabled or not
        // 32 = 0b00100000
        if lcd_control & 32 > 0 {
            // We need to check if the current scanline is wihin the windows Y Pos
            if window_y <= self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR) {
                using_window = true;
            }
        }

        // We need to determine where the tile data is located (region determined by bit 4 of lcd_control)
        // 16 = 0b00010000
        if lcd_control & 16 > 0 {
            tile_data = 0x8000; // Region of 0x8000 - 0x8FFF
        } else {
            tile_data = 0x8800; // Region of 0x8800 - 0x97FF
            // This memory region is using signed bytes as tile identifiers so set this flag
            unsigned = false;
        }

        // We need to determine which background memory region to use
        // If window is enabled, test bit 6 of LCD control, otherwise test bit 3
        // 64 = 0b01000000
        // 8 = 0b00001000
        if using_window {
            if lcd_control & 64 > 0 {
                background_memory = 0x9C00; // Region of 0x9C00 - 0x9FFF
            } else {
                background_memory = 0x9800; // Region on 0x9800 - 0x9BFF
            }
        } else {
            if lcd_control & 8 > 0 {
                background_memory = 0x9C00; // Region of 0x9C00 - 0x9FFF
            } else {
                background_memory = 0x9800; // Region on 0x9800 - 0x9BFF
            }
        }

        // The y position is used to calculate which of the 32 vertical tiles the scanline is drawing
        let mut y_pos: u8 = 0;
        if using_window {
            y_pos = self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR) - window_y;
        } else {
            y_pos = scroll_y + self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR);
        }

        // We also need to know which pixel of the current tile the scanline is on
        let tile_row: u16 = ((y_pos / 8) as u16) * 32;

        // We have 160 horizontal pixels to draw for this scanline
        for pixel in 0..160 {
            let mut x_pos: u8 = pixel + scroll_x;

            // If using the window right now, translate the x pos to window space
            if using_window {
                if pixel >= window_x {
                    x_pos = pixel - window_x;
                }
            }

            // We want to determine which tile this pixel is in - recall each tile is 8x8 pixels
            let tile_col: u16 = (x_pos / 8) as u16;
            let mut tile_num: u16 = 0;

            // We need to get the tile identity number. Based on region of data though, it might be signed or unsigned
            let tile_address: u16 = background_memory + tile_row + tile_col;
            let tile_num: u16 = self.mmu.read_memory(&(tile_address as usize)) as u16;
            let signed_tile_num = tile_num as i16;

            // Deduce where the tile identifier is in memory
            let mut tile_location = tile_data.clone();

            if unsigned {
                tile_location += tile_num * 16;
            } else {
                tile_location += ((signed_tile_num + 128) * 16) as u16;
            }

            // Find the correct vertical line we're on of the tile to get the tile data from memory
            // Each line also takes up two bytes of memory
            let line: u16 = ((y_pos % 8) * 2) as u16;
            let data_1 = self.mmu.read_memory(&((tile_location + line) as usize));
            let data_2 = self.mmu.read_memory(&((tile_location + line + 1) as usize));

            // Get the appropriate bit to determine color from the data
            // An 8-bit line of pixels has colour determined like this example
			// pixel# = 1 2 3 4 5 6 7 8
			// data 2 = 1 0 1 0 1 1 1 0
			// data 1 = 0 0 1 1 0 1 0 1
			// Pixel 1 colour id: 10
			// Pixel 2 colour id: 00
			// Pixel 3 colour id: 11
			// Pixel 4 colour id: 01
			// Pixel 5 colour id: 10
			// Pixel 6 colour id: 11
			// Pixel 7 colour id: 10
			// Pixel 8 colour id: 01
            let mut color_bit: i8 = (x_pos % 8) as i8;
            color_bit -= 7;
            color_bit *= -1;

            // We need to combine the two bytes of data to get the color ID for the pixel
            let mut color_num = (data_2 >> color_bit) & 1;
            color_num <<= 1;
            color_num |= (data_1 >> color_bit) & 1;

            // Get colour as a string, the colour palette is in memory 0xFF47
            let color = self.get_color(&color_num, &utils::COLOR_PALLETTE_ADDR);
            let mut red: u8 = 0;
            let mut green: u8 = 0;
            let mut blue: u8 = 0;

            // Setup our RGB values we want based on the color string
            if color == "white" {
                red = 255;
                green = 255;
                blue = 255;
            } else if color == "dark_gray" {
                red = 0xCC;
                green = 0xCC;
                blue = 0xCC;
            } else if color == "light_gray" {
                red = 0x77;
                green = 0x77;
                blue = 0x77;
            }

            let finaly = self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR);

            // safety check to make sure what im about
            // to set is int the 160x144 bounds
            if (finaly < 0) || (finaly > 143) || (pixel < 0) || (pixel > 159) {
                continue;
            }

            self.screen_data[((pixel * 160 + finaly) * 1) as usize] = red;
            self.screen_data[((pixel * 160 + finaly) * 2) as usize] = green;
            self.screen_data[((pixel * 160 + finaly) * 3) as usize] = blue;
        }
    }

    fn render_sprites(&mut self, lcd_control: &u8) {
        // Sprite data is located at 0x8000-0x8FFF
		// Sprite attributes are located at 0xFE00-0xFE9F and in this region
		// each sprite has 4 bytes of attributes. These are what are in each byte
		// of sprite attributes
		// 0: Sprite Y Position: Position of the sprite on the Y axis of the
		//    viewing display minus 16
		// 1: Sprite X Position: Position of the sprite on the X axis of the
		//    viewing display minus 8
		// 2: Pattern number: This is the sprite identifier used for looking up
		//    the sprite data in memory region 0x8000-0x8FFF
		// 3: Attributes: These are the attributes of the sprite

        // The size of the sprite is determined by bit 2 of LCD control
        // 4 == 0b00000100
        let is_8_by_16 = lcd_control & 4 > 0;

        // There are 40 sprite tiles. Loop through all of them and if they are visible and intercepting with
        // the current scanline, we can draw them
        for sprite in 0..40 {
            // get Index offset of sprite attributes. Remember there are 4 bytes
			// of attributes per sprite
            let index = sprite * 4;

            let y_pos = self.mmu.read_memory(&(utils::SPRITE_ATTRIBUTE_ADDR + index)) - 16;
            let x_pos = self.mmu.read_memory(&(utils::SPRITE_ATTRIBUTE_ADDR + index + 1)) - 8;
            let tile_location = self.mmu.read_memory(&(utils::SPRITE_ATTRIBUTE_ADDR + index + 2));
            let attributes = self.mmu.read_memory(&(utils::SPRITE_ATTRIBUTE_ADDR + index + 3));

            // The following are what the bits represent in the attributes
			// Bit7: Sprite to Background Priority
			// Bit6: Y flip
			// Bit5: X flip
			// Bit4: Palette number. 0 then it gets it palette from 0xFF48 otherwise 0xFF49
			// Bit3: Not used in standard gameboy
			// Bit2-0: Not used in standard gameboy
            let y_flip = attributes & 64 > 0;
            let x_flip = attributes & 32 > 0;

            let mut sprite_height = 8;
            if is_8_by_16 {
                sprite_height = 16;
            }

            let current_scanline = self.mmu.read_memory(&utils::CURRENT_SCANLINE_ADDR);

            // determine if the sprite intercepts with the scanline
			if (current_scanline >= y_pos) && (current_scanline < (y_pos + sprite_height)) {
                let mut line: i8 = (current_scanline - y_pos) as i8;

                // If we are flipping the sprite vertically (y_flip) read the sprite in backwards
                if y_flip {
                    line -= sprite_height as i8;
                    line *= -1;
                }

                // Similar process as for tiles
				line *= 2;
				let tile_data_address: u16 = (0x8000 + (tile_location * 16) as u16) + (line as u16); // TODO THIS MIGHT BE VERY WRONG - CASTING TO UNSIGNED MIGHT MESS UP THE VALUE
				let data_1 = self.mmu.read_memory(&(tile_data_address as usize));
				let data_2 = self.mmu.read_memory(&((tile_data_address + 1) as usize));

                // its easier to read in from right to left as pixel 0 is
				// bit 7 in the colour data, pixel 1 is bit 6 etc...
                for tile_pixel in 7..=0 {
                    let mut color_bit: i8 = tile_pixel.clone();

                    // Read the sprite backwards for the x axis
                    if x_flip {
                        color_bit -= 7;
                        color_bit *= -1;
                    }

                    // Carry on similarily as for tiles
                    // We need to combine the two bytes of data to get the color ID for the pixel
                    let mut color_num = (data_2 >> color_bit) & 1;
                    color_num <<= 1;
                    color_num |= (data_1 >> color_bit) & 1;

                    // Get colour as a string, the colour palette is in memory 0xFF47
                    let color = self.get_color(&color_num, &utils::COLOR_PALLETTE_ADDR);
                    let mut red: u8 = 0;
                    let mut green: u8 = 0;
                    let mut blue: u8 = 0;

                    // Setup our RGB values we want based on the color string
                    if color == "white" {
                        red = 255;
                        green = 255;
                        blue = 255;
                    } else if color == "dark_gray" {
                        red = 0xCC;
                        green = 0xCC;
                        blue = 0xCC;
                    } else if color == "light_gray" {
                        red = 0x77;
                        green = 0x77;
                        blue = 0x77;
                    }

                    let mut x_pix = 0 - tile_pixel;
                    x_pix += 7;

                    let pixel = ((x_pos as i8) + x_pix) as u8;

                    // sanity check
                    if (current_scanline < 0) || (current_scanline > 143)|| (pixel < 0) || (pixel > 159) {
                        continue;
                    }

                    self.screen_data[((pixel * 160 + current_scanline) * 1) as usize] = red;
                    self.screen_data[((pixel * 160 + current_scanline) * 2) as usize] = green;
                    self.screen_data[((pixel * 160 + current_scanline) * 3) as usize] = blue;
                }
            }
        }
    }

    fn get_color(&self, color_num: &u8, pallette_addr: &usize) -> &str {
        let pallette = self.mmu.read_memory(pallette_addr);

        let mut hi = 0;
        let mut lo = 0;

        if *color_num == 0 {
            hi = 1;
            lo = 0;
        } else if *color_num == 1 {
            hi = 3;
            lo = 2;
        } else if *color_num == 2 {
            hi = 5;
            lo = 4;
        } else if *color_num == 3 {
            hi = 7;
            lo = 6;
        }

        // Using the pallette, fetch the colour
        let mut color;
        color = ((pallette >> hi) & 1) << 1;
        color |= (pallette >> lo) & 1;

        match color {
            1 => "light_gray",
            2 => "dark_gray",
            3 => "black",
            _ => "white"
        }
    }
}
