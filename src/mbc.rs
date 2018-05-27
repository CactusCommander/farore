// Memory controllers


pub trait MemoryBankController {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

/// Memory Map
///   0000-3FFF   16KB ROM Bank 00     (in cartridge, fixed at bank 00)
///   4000-7FFF   16KB ROM Bank 01..NN (in cartridge, switchable bank number)
///   8000-9FFF   8KB Video RAM (VRAM) (switchable bank 0-1 in CGB Mode)
///   A000-BFFF   8KB External RAM     (in cartridge, switchable bank, if any)
///   C000-CFFF   4KB Work RAM Bank 0 (WRAM)
///   D000-DFFF   4KB Work RAM Bank 1 (WRAM)  (switchable bank 1-7 in CGB Mode)
///   E000-FDFF   Same as C000-DDFF (ECHO)    (typically not used)
///   FE00-FE9F   Sprite Attribute Table (OAM)
///   FEA0-FEFF   Not Usable
///   FF00-FF7F   I/O Ports
///   FF80-FFFE   High RAM (HRAM)
///   FFFF        Interrupt Enable Register
struct GBMemory {
    // The memory bank controller on the current cart
    mbc: Box<MemoryBankController>,

    // Video ram
    vram: [u8; 0x2000],

    // Work Ram (bank 0 and 1)
    wram: [u8; 0x2000],

    // Sprite Attribute Table (OAM)
    // dunno what goes here.

    // I/O ports
    // unimplemented

    // High RAM (HRAM)
    hram: [u8; 0x80],
}

impl GBMemory {
    fn new(mbc: Box<MemoryBankController>) -> Self {
        GBMemory {
            mbc,
            vram: Default::default(),
            wram: Default::default(),
            hram: Default::default(),
        }
    }

    fn read(&self, address: u16) -> u8 {
        let addr = address as usize;
        match address {
            0x0000..0x8000 => self.mbc.read(address),
            0x8000..0xA000 => self.vram[addr-0x8000],
            0xA000..0xC000 => self.mbc.read(address),
            0xC000..0xE000 => self.wram[addr-0xC000],
            0xE000..0xFE00 => self.wram[addr-0xE000],
            0xFE00..0xFEA0 => unimplemented!(), // Sprite attribute table
            0xFEA0..0xFF00 => unimplemented!(), // unusable
            0xFF00..0xFF80 => unimplemented!(), // I/O ports
            0xFF80..0xFFFE => self.hram[addr-0xFF80],
            0xFFFF         => unimplemented!(), // Interrupt Enable Register
        }
    }

    fn write(&self, address: u16, value: u8) {
        let addr = address as usize;
        match address {
            0x0000..0x8000 => self.mbc.write(address, value),
            0x8000..0xA000 => self.vram[addr-0x8000] = value,
            0xA000..0xC000 => self.mbc.write(address, value),
            0xC000..0xE000 => self.wram[addr-0xC000] = value,
            0xE000..0xFE00 => self.wram[addr-0xE000] = value,
            0xFE00..0xFEA0 => unimplemented!(), // Sprite attribute table
            0xFEA0..0xFF00 => unimplemented!(), // unusable
            0xFF00..0xFF80 => unimplemented!(), // I/O ports
            0xFF80..0xFFFE => self.hram[addr-0xFF80] = value,
            0xFFFF         => unimplemented!(), // Interrupt Enable Register
        }
    }
}

trait Ram {
    fn read(&self, bank: u8, address: u16) -> u8;
    fn write(&mut self, bank: u8, address: u16, value: u8);

    fn serialize(&self) -> Vec<u8>;
}

struct Ram2kb {
    memory: [u8; 0x800]
}

impl Ram2kb {
    fn new() -> Self {
        Ram2kb {
            memory: [0; 0x800]
        }
    }

    fn load(mem: &[u8]) -> Self {
        unimplemented!()
    }
}

impl Ram for Ram2kb {
    fn read(&self, bank: u8, address: u16) -> u8 {
        let addr = address as usize;
        if addr > self.memory.len() {
            panic("Attempted to read memory outside of range of RAM bank");
        }
        if bank != 0 {
            panic("Attempted to read memory from bank {} from bankless ram", bank);
        }
        self.memory[addr]
    }

    fn write(&mut self, bank: u8, address: u16, value: u8) {
        let addr = address as usize;
        if addr > self.memory.len() {
            panic("Attempted to write memory outside of range of RAM bank");
        }
        if bank != 0 {
            panic("Attempted to write memory to bank {} from bankless ram", bank);
        }
        self.memory[addr] = value;
    }

    fn serialize(&self) -> _ {
        unimplemented!()
    }
}

//pub struct UnswitchedController {
//    memory: [u8; ::std::u16::MAX]
//}
//
//impl MemoryBankController for UnswitchedController {
//    fn read(&self, address: u16) -> u8{
//        self.memory[address as usize]
//    }
//
//    fn write(&mut self, address: u16, value: u8) {
//        self.memory[address as usize] = value
//    }
//}

pub struct MBC1 {
    // The first bank is always mapped to 0x0-0x3FFF
    // each subsequent bank may be mapped to 0x4000-0x7FFF
    // Note that banks 0x20, 0x40, and 0x60 cannot be used.  When attempting to map these
    // banks, switch to bank 0x21, 0x41, and 0x61 respectively.
    // Similarly, when attempting to map bank 0, map bank 1 instead.  Bank 0 is always mapped.
    rom_banks: [[u8; 0x4000]; 0x80],

    // Writing to 0x2000-0x3FFF takes the lower 5 bits and uses them for bank selection
    // so in the range of 0x01-0x1F (inclusive).  Writing 0x00 also selects 0x01.
    rom_bank_number: u8,

    // External ram banks on they cart itself.
    // if the cart has a 2kb bank, its mapped to 0xA000-0xA7FF
    // if the cart has an 8kb bank, its mapped to 0xA000-0xBFFF
    // if the cart has a 32kb bank, its split into 4 banks and mapped to 0xA000-0xBFFF
    ram_bank: Box<Ram>,
    ram_bank_number: u8,
    ram_write_enabled: bool,

    // Writing to 0x4000-0x5FFF selects the RAM bank in the range of 0x00-0x03 (inclusive)
    // The same range may also be used to switch modoes to chosing the upper two bits of the
    // ROM bank selection.

    // Writing to 0x6000-0x7FFF performs previously described mode select.  This is a 1 bit
    // register.
    // Writing 0x00 switches to ROM banking mode (default)
    // Writing 0x01 switches to RAM banking mode
    // The program may freely switch between both modes, the only limitiation is that only
    // RAM Bank 00h can be used during Mode 0, and only ROM Banks 00-1Fh can be used during Mode 1.
    is_rom_banking_mode: bool,
}

impl MBC1 {
    fn new(ram: Box<Ram>) -> Self {
        MBC1 {
            rom_banks: Default::default(),
            rom_bank_number: 1,  // Rom bank zero cannot be mapped twice, so default to 1
            ram_bank: ram,
            ram_bank_number: 0,
            ram_write_enabled: false,
            is_rom_banking_mode: true,
        }
    }

    fn set_rom_bank(&mut self, bank: u8) {
        let real_bank = match bank {
            0x00 => 0x01,
            0x20 => 0x21,
            0x40 => 0x41,
        };
        self.rom_bank_number = real_bank;
    }
}

impl MemoryBankController for MBC1 {
    fn read(&self, address: u16) -> u8 {
        let addr = address as usize;
        match address {
            0x0000..0x4000 => self.rom_banks[0][addr],
            0x4000..0x8000 => self.rom_banks[self.rom_bank_number as usize][addr - 0x4000],
            0xA000..0xC000 => self.ram_bank.read(self.ram_bank_number, addr - 0xA000),
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        let addr = address as usize;
        match address {
            // Mask lower 4 bits, looking for 0xA.  0xA enables writing, any other
            // value disables writing
            0x0000..0x2000 => self.ram_write_enabled = value & 0xF == 0xA,

            // Select the lower 5 bits and use them to begin selecting the rom bank
            0x2000..0x4000 => {
                let oldnum = self.rom_bank_number & 0xE0;
                let req = value & 0x1F;
                self.set_rom_bank(oldnum | req);
            },

            // If in rom banking mode, then this 2 bit register is used to select the upper
            // two bits of the rom bank number
            // If in ram banking mode, then this 2 bit register is used to select the current
            // ram bank in range 0x00-0x03
            0x4000..0x6000 => {
                let mask = value & 0x3;
                if self.is_rom_banking_mode  {
                    self.oldval = self.rom_bank_number & 0x1F;
                    self.set_rom_bank(oldval | (mask << 5));
                } else {
                    self.ram_bank_number = mask;
                }
            },

            // This 1 bit register controls the behavior of the above 2 bit register.
            // 0x00 => switch to rom banking mode (default)
            // 0x01 => switch to ram banking mode
            0x6000..0x8000 => {
                // insufficient documentation here
                match value {
                    0x00 => {
                        // rom banking mode
                        self.is_rom_banking_mode = true;
                        self.ram_bank_number = 0;
                    },
                    0x01 => {
                        // ram banking mode
                        self.is_rom_banking_mode = false;
                        self.rom_bank_number = self.rom_bank_number & 0x1F;
                    },
                    _ => unreachable!(),
                }
            },

            0xA000..0xC000 => {
                if !self.ram_write_enabled {
                    panic!("Attempted to write ram while ram writing is disabled");
                }
                self.ram_bank.write(self.ram_bank_number, address - 0xA000, value);
            },
            _ => unreachable!(),
        }
    }
}