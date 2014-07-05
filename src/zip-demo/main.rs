#![crate_id = "zip-demo"]

extern crate zip;

use std::{os, io};

fn main() {
    let args = os::args();

    // open a zip archive
    let zip_path = Path::new(args.as_slice()[1].as_slice());
    let in_stream = io::File::open(&zip_path).unwrap();
    let mut z = zip::ZipReader::new(in_stream).unwrap();

    // list files in archive
    for i in z.iter() {
        let (year, month, day, hour, minute, second) = i.last_modified_datetime;
        println!("{} => {} bytes, {} bytes compressed, last modified: {}/{}/{} {}:{}:{}, encrypted: {}, CRC32={:#08x}",
            i.name, i.uncompressed_size, i.compressed_size, year, month, day, hour, minute, second, i.is_encrypted, i.crc32);
    }

    // if we have two arguments, extract file
    if args.len() > 2 {
        let dest_path = Path::new(args.as_slice()[2].as_slice());
        let mut out_stream = io::File::create(&dest_path).unwrap();
        let f = z.get_file_info(args.as_slice()[2].as_slice()).unwrap();
        let _ = z.extract(&f, &mut out_stream);
    }
}

