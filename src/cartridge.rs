use std::num::Wrapping;
use std::io::Write;
use itertools;

static NINTENDO_BITMAP_EXPECTED: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

#[derive(Debug, Copy, Clone)]
enum GameboyRegionCode {
    Japan,    // 0x00
    NonJapan, // 0x01
    Invalid(u8),
}

impl GameboyRegionCode {
    fn new(byte: u8) -> Self {
        match byte {
            0x00 => GameboyRegionCode::Japan,
            0x01 => GameboyRegionCode::NonJapan,
            x    => GameboyRegionCode::Invalid(x),
        }
    }
}

#[derive(Debug)]
enum GameboyColorFlag {
    Undefined,           // 0x00.  On older cartridges, this byte is part of the title.
    BackwardsCompatible, // 0x80
    GBCOnly,             // 0xC0
    Invalid(u8),
}

impl GameboyColorFlag {
    fn new(byte: u8) -> Self {
        match byte {
            0x00 => GameboyColorFlag::Undefined,
            0x80 => GameboyColorFlag::BackwardsCompatible,
            0xC0 => GameboyColorFlag::GBCOnly,
            x => GameboyColorFlag::Invalid(x),
        }
    }
}

#[derive(Debug)]
enum SuperGameboyFeatureFlag {
    Unsupported, // 0x00
    Supported,   // 0x03
    Invalid(u8),
}

impl SuperGameboyFeatureFlag {
    fn new(byte: u8) -> Self {
        match byte {
            0x00 => SuperGameboyFeatureFlag::Unsupported,
            0x03 => SuperGameboyFeatureFlag::Supported,
            x => SuperGameboyFeatureFlag::Invalid(x),
        }
    }
}

fn calculate_header_checksum(buf: &Vec<u8>) -> u8 {
    // x=0:FOR i=0134h TO 014Ch:x=x-MEM[i]-1:NEXT
    buf.iter().skip(0x0134).take(0x014C - 0x0134 + 1).cloned()
        .fold(Wrapping(0u8), |acc, x| acc - Wrapping(x) - Wrapping(1u8)).0
}

fn calculate_global_checksum(buf: &Vec<u8>) -> u16 {
    let iter = buf.iter().cloned().enumerate().filter_map(|(i, x)| {
        match i {
            0x014E => None,
            0x014F => None,
            _ => Some(x),
        }
    });

    return iter.fold(Wrapping(0u16), |acc, x| acc + Wrapping(x as u16)).0;
}

pub struct GameboyProgramMeta {
    pub name: String,  // On newer games the name is clamped to 9 chars.  Extra space is used for manufacturer code.
    pub manufacturer_code: [u8; 4],
    pub licensee_code: Vec<u8>,  // Newer games are 0x0144-0x0145.  OIlder games are 0x14B
    color_flag: GameboyColorFlag, // 0x80 = Backwards compatible with non-CGB, 0xC0 = CGB only.
    super_gameboy_flag: SuperGameboyFeatureFlag, // 0x00 = no SGB, 0x03 = SGB
    features_flag: u8, // 0x0147, Cartridge Type.  Indicates extra hardware on cartridge.
    cartridge_size_indicator: u8,  // Rom size uses this through a translation table times 32k
    ram_size_indicator: u8,  // Again uses a translation table.  Size of cold storage on cartridge
    region_code: GameboyRegionCode, // 0x00 = japanese, 0x01 = non-japanese.
    program_version_number: u8,
    header_checksum: u8, // Game will not boot if this fails. pseudocode: x=0:FOR i=0134h TO 014Ch:x=x-MEM[i]-1:NEXT
    global_checksum: u16, // Not checked by the hardware.  OK if this fails.

    header_checksum_calculated: u8,
    global_checksum_calculated: u16,
    nintendo_bitmap: [u8; 48],
    pub program_size: usize,
}

impl GameboyProgramMeta {
    pub fn new(program: &Vec<u8>) -> GameboyProgramMeta {

        let name: String = program.iter().skip(0x0134).take(0x0143 - 0x0134 + 1).take_while(|&x| *x != 0).map(|&x| x as char).collect();

        // older carts have a licensee code at 0x014B, but newer carts reserve 2 bytes for it at
        // 0x0144 and set the old licensee code to 0x33 to indicate the newer licensee code form.
        let l_code = match program[0x014B] {
            0x33 => vec![program[0x0144], program[0x0145]],
            x    => vec![x]
        };

        // Each cart must have the nintendo logo copied bit-for-bit at 0x0104-0x0133
        // Failing this assertion causes the gameboy to halt.
        let mut logo = [0; 48];
        for index in (0..48) {
            logo[index] = program[0x104 + index];
        }


        GameboyProgramMeta {
            name: name,
            manufacturer_code: [program[0x013F], program[0x0140], program[0x141], program[0x142]],
            licensee_code: l_code,
            color_flag: GameboyColorFlag::new(program[0x0143]),
            super_gameboy_flag: SuperGameboyFeatureFlag::new(program[0x0146]),
            features_flag: program[0x0147],
            cartridge_size_indicator: program[0x0148],
            ram_size_indicator: program[0x0149],
            region_code: GameboyRegionCode::new(program[0x014A]),
            program_version_number: program[0x014C],
            header_checksum: program[0x014D],
            global_checksum: program[0x014E] as u16 | ((program[0x014F] as u16) << 8usize),

            header_checksum_calculated: calculate_header_checksum(&program),
            global_checksum_calculated: calculate_global_checksum(&program),
            nintendo_bitmap: logo,
            program_size: program.len(),
        }
    }

    pub fn is_valid_logo(&self) -> bool {
        itertools::equal(self.nintendo_bitmap.iter(), NINTENDO_BITMAP_EXPECTED.iter())
    }

    pub fn is_valid_header(&self) -> bool {
        self.header_checksum == self.header_checksum_calculated
    }

    pub fn is_valid_program(&self) -> bool {
        // This checks the global_checksum against the rest of the file.  A failure does not
        // mean that the program will not execute, just that it doesn't match the advertised
        // checksum.
        self.global_checksum == self.global_checksum_calculated
    }

    pub fn is_runable(&self) -> bool {
        // The gameboy has a place on the rom for a full program checksum, but does not
        // validate the checksum, instead opting to ignore it.  Thus a runnable rom only needs to
        // contain a valid header checksum and a valid logo.
        self.is_valid_header() && self.is_valid_logo()
    }

    // pub fn declared_size(&self) -> usize {
    //     match self.cartridge_size_indicator {
    //         0x00 => 32 * 1024,
    //         0x01 => 64 * 1024,
    //         0x0
    //     }
    // }

    pub fn print_debug(&self, writer: &mut Write) {
        let test = |x| -> &str {if x {"OK"} else {"FAILED"}};

        writeln!(writer, "name: {}", self.name).ok();
        writeln!(writer, "size: {}", self.program_size).ok();
        writeln!(writer, "manufacturer code: {:?}", self.manufacturer_code).ok();
        writeln!(writer, "licensee code: {:?}", self.licensee_code).ok();
        writeln!(writer, "color flag: {:?}", self.color_flag).ok();
        writeln!(writer, "super flag: {:?}", self.super_gameboy_flag).ok();
        writeln!(writer, "features flag: {:?}", self.features_flag).ok();
        writeln!(writer, "size indicator: {:?}", self.cartridge_size_indicator).ok();
        writeln!(writer, "ram indiciator: {:?}", self.ram_size_indicator).ok();
        writeln!(writer, "region code: {:?}", self.region_code).ok();
        writeln!(writer, "version number: {:?}", self.program_version_number).ok();
        writeln!(writer, "header checksum: Declared({0:?}) Calculated({1:?})", self.header_checksum, self.header_checksum_calculated).ok();
        writeln!(writer, "global checksum: Declared({0:?}) Calculated({1:?})", self.global_checksum, self.global_checksum_calculated).ok();
        writeln!(writer, "logo test: {}", test(self.is_valid_logo())).ok();
        writeln!(writer, "header test: {}", test(self.is_valid_header())).ok();
        writeln!(writer, "program test: {}", test(self.is_valid_program())).ok();
        writeln!(writer, "runable test: {}", test(self.is_runable())).ok();
    }
}
