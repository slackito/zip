#![allow(unstable)]

extern crate flate;

pub use self::fileinfo::{CompressionMethod, FileInfo};
pub use self::reader::ZipReader;

mod crc32;
pub mod maybe_utf8;
#[macro_use] pub mod error;
pub mod format;
pub mod fileinfo;
pub mod reader;

