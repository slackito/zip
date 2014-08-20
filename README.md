# rust-zip [![Build Status](https://travis-ci.org/slackito/zip.svg)](https://travis-ci.org/slackito/zip)

A simple rust library to read and write Zip archives, which is also my pet project for learning Rust.
At the moment you can list the files in a Zip archive, as well as extracting them if they are either stored
(uncompressed) or deflated, but I plan to add write support soon.

A simple example
----------------

```rust
extern crate zip;

use std::{os, io};

fn main() {
    let args = os::args();

    // open a zip archive
    let zip_path = Path::new(args.as_slice()[1].as_slice());
    let mut z = zip::ZipReader::open(&zip_path).unwrap();

    // list files in archive
    for i in z.files() {
        let (year, month, day, hour, minute, second) = i.last_modified_datetime;
        println!("{} => {} bytes, {} bytes compressed, last modified: {}/{}/{} {}:{}:{}, encrypted: {}, CRC32={:#08x}",
            i.name, i.uncompressed_size, i.compressed_size, year, month, day, hour, minute, second, i.is_encrypted, i.crc32);
    }

    // if we have two arguments, extract file
    if args.len() > 2 {
        let dest_path = Path::new(args.as_slice()[2].as_slice());
        let mut out_stream = io::File::create(&dest_path).unwrap();
        let f = z.info(args.as_slice()[2].as_slice()).unwrap();
        let _ = z.extract(&f, &mut out_stream);
    }
}
```

TODO
----

- Learn more Rust
- Write support
- Create a proper set of tests
- Support advanced features (more compression methods, ZIP64, encryption, multiple volumes...)

