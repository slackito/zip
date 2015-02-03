# rust-zip [![Build Status](https://travis-ci.org/slackito/zip.svg)](https://travis-ci.org/slackito/zip)

A simple rust library to read and write Zip archives, which is also my pet project for learning Rust.
At the moment you can list the files in a Zip archive, as well as extracting them if they are either stored
(uncompressed) or deflated, but I plan to add write support soon.

A simple example
----------------

```rust
extern crate zip;

use std::old_io::File;
use zip::ZipReader;
use zip::fileinfo::FileInfo;

fn main() {
    let args = std::os::args();
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

fn output_file(file: &str)->File{
    do_or_die!(File::create(&Path::new(file)))
}

fn zipped_file_info(zip: &mut ZipReader<File>, file: &str) -> FileInfo{
    do_or_die!(zip.info(file))
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
    let mut out = output_file(file);
    let info = zipped_file_info(zip, file);
    do_or_die!(zip.extract(&info, &mut out));
}

fn usage(this: &str)->(){
    println!("Usage: {} [file.zip] [file_to_extract]", this);
}

```

TODO
----

- Learn more Rust
- Write support
- Create a proper set of tests
- Support advanced features (more compression methods, ZIP64, encryption, multiple volumes...)

