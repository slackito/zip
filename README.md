# rust-zip

A simple rust library to read and write Zip files. Currently only reading them is supported,
and it's probably broken :)

A simple example
----------------

```rust
extern mod zip;

use std::os;
use std::rt::io::*;
use zip::*;

fn main() {
    let args = os::args();

    // open a zip archive
    let zip_path = Path(args[1]);
    let mut in_stream = file::open(&zip_path, Open, Read).unwrap();
    let mut z = ZipReader::new(in_stream).unwrap();

    // list files in archive
    for i in z.iter() {
        let (year, month, day) = i.last_modified_date;
        let (hour, minute, second) = i.last_modified_time;
        println!("{} => {} bytes, {} bytes compressed, last modified: {}/{}/{} {}:{}:{}, encrypted: {}, CRC32={:#08x}",
            i.name, i.uncompressed_size, i.compressed_size, year, month, day, hour, minute, second, i.is_encrypted, i.crc32);
    }

    // if we have two arguments, extract file
    if (args.len() > 2) {
        let dest_path = Path(args[2]);
        let mut out_stream = file::open(&dest_path, CreateOrTruncate, Write).unwrap();
        let f = z.get_file_info(args[2]).unwrap();
        z.extract(&f, &mut out_stream);
    }
}
```

TODO
----

- Learn Rust
- Check CRC32 when extracting files
- Add support for writing ZIP files
- Support advanced features (more compression methods, ZIP64, encryption, multiple volumes...)

