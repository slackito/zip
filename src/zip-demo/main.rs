#![feature(core, os, io, path)]

extern crate zip;

use std::os;
use std::old_io as io;

fn main() {
    let args = os::args();

    // open a zip archive
    let zip_path = Path::new(&args[1][]);
    let mut z = zip::ZipReader::open(&zip_path).unwrap();

    // list files in archive
    for i in z.files() {
        let (year, month, day, hour, minute, second) = i.last_modified_datetime;
        println!("{} => {} bytes, {} bytes compressed, last modified: {}/{}/{} {}:{}:{}, encrypted: {}, CRC32={:#08x}",
            i.name, i.uncompressed_size, i.compressed_size, year, month, day, hour, minute, second, i.is_encrypted, i.crc32);
    }

    // if we have two arguments, extract file
    if args.len() > 2 {
        let dest_path = Path::new(&args[2][]);
        let mut out_stream = io::File::create(&dest_path).unwrap();
        let f = z.info(&args[2][]).unwrap();
        let _ = z.extract(&f, &mut out_stream);
    }
}

