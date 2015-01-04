//! A list of possible errors.

#![macro_escape]

use std::fmt;
use std::io::IoError;

/// A list of possible errors. This is a supetset of `std::Io::IoError`.
#[derive(PartialEq,Clone)]
pub enum ZipError {
    IoError(IoError),
    NotAZipFile,
    CrcError,
    FileNotFoundInArchive,
    InvalidSignature(u32),
    NonUTF8Field,
    TooLongField,
}

impl fmt::Show for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ZipError::IoError(ref e) => write!(f, "{}", e),
            ZipError::NotAZipFile => write!(f, "not a ZIP file"),
            ZipError::CrcError => write!(f, "CRC mismatch"),
            ZipError::FileNotFoundInArchive => write!(f, "file not found in archive"),
            ZipError::InvalidSignature(magic) => write!(f, "invalid ZIP signature {:#08x}", magic),
            ZipError::NonUTF8Field =>
                write!(f, "file name or comment is set to UTF-8 encoded but it isn't"),
            ZipError::TooLongField =>
                write!(f, "file name, comment or extra field is too long (> 64KB)"),
        }
    }
}

pub type ZipResult<T> = Result<T, ZipError>;

macro_rules! try_io {
    ($e:expr) => (try!($e.map_err(::error::ZipError::IoError)))
}

