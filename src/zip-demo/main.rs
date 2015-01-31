#![feature(core, os, io, path)]

extern crate zip;

use std::os;
use std::old_io as io;
use zip::ZipReader;

fn main() {
    let args = os::args();
    match args.len()
    {
        2 => list(&mut zip_file(&args[1][])),
        3 => extract(&mut zip_file(&args[1][]), &args[2][]),
        _ => usage(&args[0][])
    }
}

fn zip_file(path: &str) -> ZipReader<io::File>{
    zip::ZipReader::open(&Path::new(path)).unwrap()
}

fn list(reader: &mut ZipReader<io::File>)->(){
    for file in reader.files(){
        let (year, month, day, hour, minute, second) = file.last_modified_datetime;
        let mod_time = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);
        println!("{} ({}): bytes: {:10}, compressed: {:10}",
            file.name, mod_time, file.compressed_size, file.uncompressed_size);
    }
}

fn extract(zip: &mut ZipReader<io::File>, file: &str)->(){
    let mut stream = io::File::create(&Path::new(file)).unwrap();
    let info = zip.info(file).unwrap();
    zip.extract(&info, &mut stream).unwrap();
}

fn usage(this: &str)->(){
    println!("Usage: {} [file.zip] [file_to_extract]" , this);
}
