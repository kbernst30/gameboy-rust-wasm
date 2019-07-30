use super::utils;
use super::game;

use wasm_bindgen::prelude::*;

// MEMORY INFO
//
// 0000-3FFF 16KB ROM Bank 00 (in cartridge, fixed at bank 00)
// 4000-7FFF 16KB ROM Bank 01..NN (in cartridge, switchable bank number)
// 8000-9FFF 8KB Video RAM (VRAM) (switchable bank 0-1 in CGB Mode)
// A000-BFFF 8KB External RAM (in cartridge, switchable bank, if any)
// C000-CFFF 4KB Work RAM Bank 0 (WRAM)
// D000-DFFF 4KB Work RAM Bank 1 (WRAM) (switchable bank 1-7 in CGB Mode)
// E000-FDFF Same as C000-DDFF (ECHO) (typically not used)
// FE00-FE9F Sprite Attribute Table (OAM)
// FEA0-FEFF Not Usable
// FF00-FF7F I/O Ports
// FF80-FFFE High RAM (HRAM)
// FFFF Interrupt Enable Register

pub struct Mmu {
    memory: [u8; 0x10000],

    // Joypad byte - we will use 8 bits for denoting key pressed - not the same
	// as internal memory joypad state. Just for convenience sake and for setting
	// internal memory
    joypad: u8,

    // There are two types of rom banking, MBC1 and MBC2
	// Some games don't use either and the rom bank mode is found at memory
	// location 0x147 after the game is loaded into memory (0x000 - 0x7FFF)
	// Use flags to determine which type of rom banking is being used
    mbc1: bool,
    mbc2: bool,
    rom_banking: bool,

    // Different rom banks could be loaded into second area of memory (4000 - 7FFF)
	// But memory region 0000 - 7FFF is fixed at rom bank 0. That stays loaded
	// So keep a variable that says what rom bank is loaded into the second region
    current_rom_bank: u8,

    // Memory location 0x148 tells how many RAM banks exist
	// A RAM bank is 0x2000 bytes in size and the maximum RAM banks that a game can
	// have is 4. Keep an Array variable to represent 4 RAM banks (0x8000 in size)
	// and a variable to tell us which RAM bank is being used currently (between 0 and 3)
	// RAM banking isn't used if ROM bank mode is MBC2 so currentRamBank will stay 0
    ram_banks: [u8; 0x8000],
    current_ram_bank: u8,
    enable_ram: bool,

    timer_counter: usize,

    cartridge: game::Game
}

impl Mmu {
    pub fn new(game: game::Game) -> Mmu {
        // Init Memory to all 0 and then some spots equal to the following (from Docs)
        let mut memory = [0; 0x10000];

        memory[0xFF05] = 0x00;
        memory[0xFF06] = 0x00;
        memory[0xFF07] = 0x00;
        memory[0xFF10] = 0x80;
        memory[0xFF11] = 0xBF;
        memory[0xFF12] = 0xF3;
        memory[0xFF14] = 0xBF;
        memory[0xFF16] = 0x3F;
        memory[0xFF17] = 0x00;
        memory[0xFF19] = 0xBF;
        memory[0xFF1A] = 0x7F;
        memory[0xFF1B] = 0xFF;
        memory[0xFF1C] = 0x9F;
        memory[0xFF1E] = 0xBF;
        memory[0xFF20] = 0xFF;
        memory[0xFF21] = 0x00;
        memory[0xFF22] = 0x00;
        memory[0xFF23] = 0xBF;
        memory[0xFF24] = 0x77;
        memory[0xFF25] = 0xF3;
        memory[0xFF26] = 0xF1;
        memory[0xFF40] = 0x91;
        memory[0xFF42] = 0x00;
        memory[0xFF43] = 0x00;
        memory[0xFF45] = 0x00;
        memory[0xFF47] = 0xFC;
        memory[0xFF48] = 0xFF;
        memory[0xFF49] = 0xFF;
        memory[0xFF4A] = 0x00;
        memory[0xFF4B] = 0x00;
        memory[0xFFFF] = 0x00;

        // Set the first bank into memory 0x000 - 0x7FFF
        for i in 0..0x8000 {
            memory[i] = game.read_catridge_data(i);
        }

        Mmu {
            memory,
            joypad: 7, // All bits set to 1
            mbc1: false,
            mbc2: false,
            rom_banking: true,
            current_rom_bank: 1,
            ram_banks: [0; 0x8000],
            current_ram_bank: 0,
            enable_ram: false,
            timer_counter: 1024, // Initial value, frequency 4096 (4194304/4096)
            cartridge: game
        }
    }

    pub fn determine_rom_banking_type(&mut self) {
        match self.memory[0x147] {
            1 => self.mbc1 = true,
            2 => self.mbc1 = true,
            3 => self.mbc1 = true,
            4 => self.mbc1 = true,
            5 => self.mbc1 = true,
            6 => self.mbc1 = true,
            _ => log!("no memory banking necessary")
        }
    }

    pub fn read_memory(&self, address: &usize) -> u8 {
        match *address {
            // If reading the Joypad memory byte, resolve our joypad object to what the
		    // memory should actually look like
            0xFF00                          => self.get_joypad_state(),

            // If reading from ROM bank, find actual data we want in cartridge memory
            m if m >= 0x4000 && m <= 0x7FFF => self.do_read_cartridge_data(m),

            // If reading from RAM bank
            m if m >= 0xA000 && m <= 0xBFFF => self.do_read_ram_bank(m),

            // Anything else, read normally
            _                               => self.memory[*address]
        }
    }

    pub fn write_memory(&mut self, address: &usize, data: u8) {
        match *address {
            // If address is in Game ROM Area, don't write, this is read-only
			// Handle ROM banking though
            m if m < 0x8000                => self.do_handle_banking(address, data),
            m if m >= 0xA000 && m < 0xC000 => self.do_handle_ram_banks(address, data),

            // This is the divider register and if we try and write to this,
			// it should reset to 0
            utils::DIVIDER_REGISTER_ADDR   => self.memory[*address] = 0,

            utils::TIMER_CONTROLLER_ADDR   => self.do_handle_timer_controller(data),

            // This is the register that holds the current scanline and if we try
			// to write to this, it should reset to 0
            0xFF44                         => self.memory[*address] = 0,

            // When requesting this address, a Direct Memory Access is launched
			// which is when data is copied to Sprite RAM (FE00-FE9F). This can
			// be accessed during LCD Status Mode 2
            0xFF46                         => self.do_dma_transer(data),

            // This is not usable memory. Restricted access. Don't write
            m if m >= 0xFEA0 && m < 0xFEFF => log!("Attempted to write to restricted memory - {}", m),

            // If you write to ECHO, you also have to write to RAM
            m if m >= 0xE000 && m < 0xFDFF => self.do_echo_write(address, data),

            // Anything else, write to memory
            _                              => self.do_write_data(address, data)
        }
    }

    pub fn get_clock_frequency(&self) -> u8 {
        // Clock freq is combination of 1st and 2nd bit of timer controller
        self.read_memory(&utils::TIMER_CONTROLLER_ADDR) & 0x3
    }

    pub fn set_clock_frequency(&mut self) {
        let frequency = self.get_clock_frequency();
        match frequency {
            0 => self.timer_counter = 1024, // Freq 4096
            1 => self.timer_counter = 16,   // Freq 4096
            2 => self.timer_counter = 64,   // Freq 65536
            3 => self.timer_counter = 256,  // Freq 16382
            _ => log!("Invalid value for clock frequency {}", frequency)
        }
    }

    pub fn decrease_timer_counter(&mut self, cycles: &usize) {
        self.timer_counter -= cycles;
    }

    pub fn get_timer_counter(&self) -> &usize {
        &self.timer_counter
    }

    pub fn increment_divider_register(&mut self) {
        self.memory[utils::DIVIDER_REGISTER_ADDR] += 1;
    }

    fn do_read_cartridge_data(&self, address: usize) -> u8 {
        let cartridge_address = (address - 0x4000) + ((self.current_rom_bank as usize) * 0x4000);
        self.cartridge.read_catridge_data(cartridge_address)
    }

    fn do_read_ram_bank(&self, address: usize) -> u8 {
        let resolved_address = address - 0xA000;
        self.ram_banks[resolved_address + ((self.current_ram_bank as usize) * 0x2000)]
    }

    fn do_write_data(&mut self, address: &usize, data: u8) {
        self.memory[*address] = data;
    }

    fn do_handle_banking(&mut self, address: &usize, data: u8) {
        match *address {
            // If the address is between 0x0000 and 0x2000, and ROM Banking is enabled
			// then we attempt RAM enabling
            m if m < 0x2000                => self.do_enable_ram_banking(address, data),

            // If the address is between 0x2000 and 0x4000, and ROM banking is enabled
			// then we perform a ROM bank change
            m if m >= 0x2000 && m < 0x4000 => self.do_rom_lo_bank_change(data),

            // If the address is between 0x4000 and 0x6000 then we perform either
			// a RAM bank change or ROM bank change depending on what RAM/ROM mode
			// is selected
            m if m >= 0x4000 && m < 0x6000 => self.do_rom_or_ram_bank_change(data),

            // In mbc1, rom banking is flipped depending on data to signify
			// a RAM banking change instead. If we are writing to an address
			// between 0x6000 and 0x8000 that is how we know if we should change
			// this flag or not
            m if m >= 0x6000 && m < 0x8000 => self.do_change_rom_ram_mode(data),

            // Match for edge case - do nothing
            _                              => log!("Invalid address for rom banking - {}", address)
        }
    }

    fn do_handle_ram_banks(&mut self, address: &usize, data: u8) {
        if self.enable_ram {
            let resolved_address = address - 0xA000;
            self.ram_banks[resolved_address + ((self.current_ram_bank as usize) * 0x2000)] = data;
        }
    }

    fn do_handle_timer_controller(&mut self, data: u8) {
        let current_frequency = self.get_clock_frequency();
        self.memory[utils::TIMER_CONTROLLER_ADDR] = data;
        let new_frequency = self.get_clock_frequency();

        if current_frequency != new_frequency {
            self.set_clock_frequency();
        }
    }

    fn do_dma_transer(&mut self, data: u8) {
        // DMA writes data to the Sprite Attribute Table (OAM), addresses FE00-FE9F
		// The source address of data to be written represented by the data passed in here
		// However, this value is actually the source address divided by 100. We need to
		// multiply it by 100 (to save speed, I have seen the suggestion to bit-wise shift left
		// by 8 spots instead. This is the same as multiplying by 100)

        let mut source_address = (data.checked_shl(8).unwrap_or(0)) as usize;
        for i in 0xFE00..=0xFE9F {
            let data_to_write = self.read_memory(&source_address);
            self.write_memory(&i, data_to_write);
            source_address += 1;
        }
    }

    fn do_echo_write(&mut self, address: &usize, data: u8) {
        let echo_address = *address - 0x2000;
        self.do_write_data(&echo_address, data);
        self.do_write_data(address, data);
    }

    fn do_enable_ram_banking(&mut self, address: &usize, data: u8) {
        // mbc2 says that bit 4 of the address must be 0 for RAM Banking to be enabled
        if self.mbc2 {
            // 8 == 0b1000
            if address & 8 == 1 {
                // Bit-Wise AND showed us bit 4 was 1 and not 0 so return
                log!("Bit 4 of address {} was 1 - do not enable ram banking", address);
                return;
			}

			// If lower nibble of data being written is 0xA then we enable RAM Banking
			// and if the lower nibble is 0 then it is disabled
			// var lowerNibble = data & 0xF;
            let lower_nibble = data & 0xF;
            if lower_nibble == 0xA {
                self.enable_ram = true;
            } else if lower_nibble == 0 {
                self.enable_ram = false;
            }
        }
    }

    fn do_rom_lo_bank_change(&mut self, data: u8) {
        // if mbc1, bits 0-4 are changed but not 5 and 6
		// if mbc2, bits 0-3 are changed and bits 5 and 6 are never set
        if self.mbc2 {
            self.current_rom_bank = data & 0xF; // Lower nibble (bits 0-3)
            if self.current_rom_bank == 0 {
                // This cannot be 0 as rom bank 0 is always in Memory 0000-3FFF
                self.current_rom_bank = self.current_rom_bank + 1;
            }

        } else if self.mbc2 {
            let lower_five_bits = data & 31; // 31 = 0b11111
            self.current_rom_bank &= 224; // 224 = 0b11100000 Flip off lower 5 bits for now
            self.current_rom_bank |= lower_five_bits; // Bit wise OR will give us new value for lower 5
            if self.current_rom_bank == 0 {
                // This cannot be 0 as rom bank 0 is always in Memory 0000-3FFF
                self.current_rom_bank = self.current_rom_bank + 1;
            }
        }
    }

    fn do_rom_hi_bank_change(&mut self, data: u8) {
        // Only used for mbc1, mbc2 doesn't concern itself with the upper bits
		// of the current ROM bank

        self.current_rom_bank &= 31; // 31 = 0b11111 - Flip off the upper 3 bits for now
        let new_data = data & 224; // 224 = 0b11100000 - Flip off the lower 5 bits of data
        self.current_rom_bank |= new_data; // Bit wise OR here should give us the bits we care about
        if self.current_rom_bank == 0 {
            // This cannot be 0 as rom bank 0 is always in Memory 0000-3FFF
            self.current_rom_bank = self.current_rom_bank + 1;
        }
    }

    fn do_ram_bank_change(&mut self, data: u8) {
        // Only used for mbc1 as mbc2 holds External RAM on the cartridge not in memory
		// Set RAM Bank to the lower 2 bits of the data
		self.current_ram_bank = data & 0x2;
    }

    fn do_change_rom_ram_mode(&mut self, data: u8) {
        if self.mbc1 {
            // If least significant bit of data being written is 0 then romBanking is set to true
            // otherwise it is set to false, signifying RAM banking
            // Current RAM bank should be set to 0 if romBanking is true
            let least_significant_bit = data & 0x1;
            if least_significant_bit == 0 {
                self.rom_banking = true;
                self.current_ram_bank = 0;
            } else if least_significant_bit == 1 {
                self.rom_banking = false;
            }
        }
    }

    fn do_rom_or_ram_bank_change(&mut self, data: u8) {
        if self.mbc1 {
            // no RAM banking if mbc2
            if self.rom_banking {
                self.do_rom_hi_bank_change(data);
            } else {
                self.do_ram_bank_change(data);
            }
        }
    }

    fn get_joypad_state(&self) -> u8 {
        // Our Joypad object represents this
		// Right = 0
		// Left = 1
		// Up = 2
		// Down = 3
		// A = 4
		// B = 5
		// SELECT = 6
		// START = 7

		// Actual byte is this:
		// Bit 7 - Not used
		// Bit 6 - Not used
		// Bit 5 - P15 Select Button Keys (0=Select)
		// Bit 4 - P14 Select Direction Keys (0=Select)
		// Bit 3 - P13 Input Down or Start (0=Pressed) (Read Only)
		// Bit 2 - P12 Input Up or Select (0=Pressed) (Read Only)
		// Bit 1 - P11 Input Left or Button B (0=Pressed) (Read Only)
		// Bit 0 - P10 Input Right or Button A (0=Pressed) (Read Only)

        let mut result = self.memory[0xFF00];

        // Flip the bits
        result ^= 0xFF;

        // If we are interested in the standard buttons
        // 32 == 0b00100000, 16 = 0b00010000
        if result & 32 > 0 {
            // Move the top nibble of the byte that has the standard buttons into
			// a lower nibble
			let mut top_nibble = self.joypad >> 4;
			top_nibble |= 0xF0;
			result &= top_nibble;

        } else if result & 16 > 0 {
            // Directional buttons
            let mut bottom_nibble = self.joypad & 0xF;
            bottom_nibble |= 0xF0;
            result &= bottom_nibble;
        }

        result
    }
}
