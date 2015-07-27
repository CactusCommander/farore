use std::fs::File;
use std::io::{BufReader, Read};
use std::num::Wrapping;


// static PROGRAM_BITMAP_EXPECTED: [u8; 48] = [
//     0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
//     0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
//     0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
// ];

// enum GameboyRegionCode {
//     Japan,
//     NonJapan,
// }
//
// enum GameboyColorFlag {
//     BackwardsCompatible,
//     GBCOnly,
// }
//
// enum SuperGameboyFeatureFlag {
//
// }

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

struct GameboyProgramMeta {
    name: String,  // On newer games the name is clamped to 9 chars.  Extra space is used for manufacturer code.
    manufacturer_code: [u8; 4],
    licensee_code: Vec<u8>,  // Newer games are 0x0144-0x0145.  OIlder games are 0x14B
    color_flag: u8, // 0x80 = Backwards compatible with non-CGB, 0xC0 = CGB only.
    super_gameboy_flag: u8, // 0x00 = no SGB, 0x03 = SGB
    features_flag: u8, // 0x0147, Cartridge Type.  Indicates extra hardware on cartridge.
    cartridge_size_indicator: u8,  // Rom size uses this through a translation table times 32k
    ram_size_indicator: u8,  // Again uses a translation table.  Size of cold storage on cartridge
    destination_code: u8, // 0x00 = japanese, 0x01 = non-japanese.
    program_version_number: u8,
    header_checksum: u8, // Game will not boot if this fails. pseudocode: x=0:FOR i=0134h TO 014Ch:x=x-MEM[i]-1:NEXT
    global_checksum: u16, // Not checked by the hardware

    // The rom itself.
    // program_buffer: &'a Vec<u8>,

    header_checksum_calculated: u8,
    global_checksum_calculated: u16,
    // logo: [u8; 48],
}

impl GameboyProgramMeta {
    fn new(program: &Vec<u8>) -> GameboyProgramMeta {
        let name: String = program.iter().skip(0x0134).take(0x0143 - 0x0134 + 1).take_while(|&x| *x != 0).map(|&x| x as char).collect();

        let l_code = match program[0x014B] {
            0x33 => vec![program[0x0144], program[0x0145]],
            x    => vec![x]
        };

        println!("Checksum be {0:X} {1:X}", program[0x014E], program[0x014F]);

        GameboyProgramMeta {
            name: name,
            manufacturer_code: [program[0x013F], program[0x0140], program[0x141], program[0x142]],
            licensee_code: l_code,
            color_flag: program[0x0143],
            super_gameboy_flag: program[0x0146],
            features_flag: program[0x0147],
            cartridge_size_indicator: program[0x0148],
            ram_size_indicator: program[0x0149],
            destination_code: program[0x014A],
            program_version_number: program[0x014C],
            header_checksum: program[0x014D],
            global_checksum: program[0x014E] as u16 | ((program[0x014F] as u16) << 8usize),

            header_checksum_calculated: calculate_header_checksum(&program),
            global_checksum_calculated: calculate_global_checksum(&program),
        }
    }
}

fn string_substring(buf: &Vec<u8>, sub1: usize, sub2: usize) -> String {
    buf.iter().skip(sub1).take(sub2).map(|&x| x as char).collect()
}

fn main() {
    let rom_path: String;
    match std::env::args().nth(1) {
        Some(x) => {
            println!("Opening rom {}", x);
            rom_path = x;
        },
        None => {
            println!("Requires a rom at parameter 1.");
            return;
        },
    }

    let mut rom_buf: Vec<u8> = Vec::new();
    match File::open(rom_path.clone()) {
        Ok(file) => BufReader::new(&file).read_to_end(&mut rom_buf).ok(),
        Err(..) => panic!("Unable to open file {}", rom_path),
    };

    println!("Total size: {}", rom_buf.len());
    println!("Title: {:?}", string_substring(&rom_buf, 0x134, 0x143 - 0x134));
    println!("Manufacturer Code: {:?}", string_substring(&rom_buf, 0x13F, 0x142 - 0x13F));

    let meta = GameboyProgramMeta::new(&rom_buf);
    println!("name: {}", meta.name);
    println!("manufacturer code: {:?}", meta.manufacturer_code);
    println!("licensee code: {:?}", meta.licensee_code);
    println!("color flag: {:?}", meta.color_flag);
    println!("super flag: {:?}", meta.super_gameboy_flag);
    println!("features flag: {:?}", meta.features_flag);
    println!("size indicator: {:?}", meta.cartridge_size_indicator);
    println!("ram indiciator: {:?}", meta.ram_size_indicator);
    println!("destination code: {:?}", meta.destination_code);
    println!("version number: {:?}", meta.program_version_number);
    println!("header checksum: Declared({0:?}) Calculated({1:?})", meta.header_checksum, meta.header_checksum_calculated);
    println!("gobal checksum: Declared({0:?}) Calculated({1:?})", meta.global_checksum, meta.global_checksum_calculated);
}
