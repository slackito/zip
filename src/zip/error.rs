//! A list of possible errors.

#![macro_escape]

use std::fmt;
use std::io::IoError;

/// A list of possible errors. This is a supetset of `std::Io::IoError`.
#[deriving(Eq,Clone)]
pub enum ZipError {
    IoError(IoError),
    NotAZipFile,
    CrcError,
    FileNotFoundInArchive
}

impl fmt::Show for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IoError(ref e) => e.fmt(f),
            NotAZipFile => "not a ZIP file".fmt(f),
            CrcError => "CRC mismatch".fmt(f),
            FileNotFoundInArchive => "file not found in archive".fmt(f),
        }
    }
}

pub type ZipResult<T> = Result<T, ZipError>;

macro_rules! try_io(
    ($e:expr) => (try!($e.map_err(::error::IoError)))
)

