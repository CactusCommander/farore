extern crate itertools;

mod cartridge;

use std::fs::File;
use std::io::{BufReader, Read, stdout};


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

    let meta = cartridge::GameboyProgramMeta::new(&rom_buf);
    meta.print_debug(&mut stdout());
}
