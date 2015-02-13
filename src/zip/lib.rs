#![feature(core, io, path, collections, rustc_private)]

extern crate flate;
extern crate maybe_utf8;

pub use self::fileinfo::{CompressionMethod, FileInfo};
pub use self::reader::ZipReader;

mod crc32;
#[macro_use] pub mod error;
pub mod format;
pub mod fileinfo;
pub mod reader;

