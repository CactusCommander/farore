extern crate sha1;
extern crate byteorder;

mod cart;

use std::fs::File;
use std::io::{BufReader, Read, stdout};


fn main() -> Result<(), Box<::std::error::Error>> {
    let rom_path: String;
    match std::env::args().nth(1) {
        Some(x) => {
            println!("Opening rom {}", x);
            rom_path = x;
        },
        None => {
            eprintln!("Requires a rom at parameter 1.");
            return Ok(());
        },
    }

    let mut rom_buf: Vec<u8> = Vec::new();
    match File::open(rom_path.clone()) {
        Ok(file) => BufReader::new(&file).read_to_end(&mut rom_buf).ok(),
        Err(..) => panic!("Unable to open file {}", rom_path),
    };

    let meta = cart::GameboyProgramMeta::new(&rom_buf)?;
    meta.print_debug(&mut stdout());
    Ok(())
}
