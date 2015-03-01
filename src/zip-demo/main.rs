#![feature(env, old_io, old_path)]

extern crate zip;

use std::env;
use std::old_io::File;
use zip::ZipReader;
use zip::fileinfo::FileInfo;

// this is public so that rustdoc doesn't complain
pub fn main() {
    let args: Vec<_> = env::args().collect(); // XXX
    match args.len(){
        2 => list_content(&mut zip_file(&args[1])),
        3 => extract_file(&mut zip_file(&args[1]), &args[2]),
        4 => extract_head(&mut zip_file(&args[1]), &args[2], &args[3]),
        _ => print_usage(&args[0])
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

fn output_file(file: &str)->File{
    do_or_die!(File::create(&Path::new(file)))
}

fn zip_file_info(zip: &mut ZipReader<File>, file: &str) -> FileInfo{
    do_or_die!(zip.info(file))
}

fn list_content(reader: &mut ZipReader<File>)->(){
    for file in reader.files(){
        let (year, month, day, hour, minute, second) = file.last_modified_datetime;
        let mod_time = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);
        println!("{} ({}): bytes: {:10}, compressed: {:10}",
            file.name, mod_time, file.compressed_size, file.uncompressed_size);
    }
}

fn extract_file(zip: &mut ZipReader<File>, file: &str)->(){
    let mut out = output_file(file);
    let info = zip_file_info(zip, file);
    do_or_die!(zip.extract_file(&info, &mut out));
}

fn extract_head(zip: &mut ZipReader<File>, file: &str, length: &str)->(){
    let mut out = output_file(file);
    let info = zip_file_info(zip, file);
    let len:usize = do_or_die!(length.parse());
    do_or_die!(zip.extract_first(&info, len, &mut out));
}

fn print_usage(this: &str)->(){
    println!("Usage: {} [file.zip] [file_to_extract] [bytes_to_extract]", this);
}
