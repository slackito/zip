#![feature(core, os, io, path)]

extern crate zip;

use std::os;
use std::old_io::File;
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

macro_rules! do_or_die{
    ($expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => {println!("{}",err); panic!()}
    })
}

fn zip_file(file: &str) -> ZipReader<File>{
    do_or_die!(zip::ZipReader::open(&Path::new(file)))
}

fn out_file(file: &str)->File{
    do_or_die!(File::create(&Path::new(file)))
}


fn list(reader: &mut ZipReader<File>)->(){
    for file in reader.files(){
        let (year, month, day, hour, minute, second) = file.last_modified_datetime;
        let mod_time = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);
        println!("{} ({}): bytes: {:10}, compressed: {:10}",
            file.name, mod_time, file.compressed_size, file.uncompressed_size);
    }
}

fn extract(zip: &mut ZipReader<File>, file: &str)->(){
    let mut out = out_file(file);
    let info = do_or_die!(zip.info(file));
    do_or_die!(zip.extract(&info, &mut out));
}

fn usage(this: &str)->(){
    println!("Usage: {} [file.zip] [file_to_extract]" , this);
}
