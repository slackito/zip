//! Byte container optionally encoded as UTF-8.

use std::{str, fmt};
use std::str::MaybeOwned;
use std::default::Default;
use std::path::BytesContainer;

#[deriving(Clone)]
pub enum MaybeUTF8 {
    UTF8(String),
    Bytes(Vec<u8>),
}

impl MaybeUTF8 {
    pub fn new() -> MaybeUTF8 {
        UTF8(String::new())
    }

    pub fn from_str(s: String) -> MaybeUTF8 {
        UTF8(s)
    }

    pub fn from_bytes(v: Vec<u8>) -> MaybeUTF8 {
        Bytes(v)
    }

    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        match *self {
            UTF8(ref s) => s.as_bytes(),
            Bytes(ref v) => v.as_slice(),
        }
    }

    pub fn as_str<'a>(&'a self) -> Option<&'a str> {
        match *self {
            UTF8(ref s) => Some(s.as_slice()),
            Bytes(ref v) => str::from_utf8(v.as_slice()),
        }
    }

    pub fn map_as_maybe_owned<'a>(&'a self,
                                  as_maybe_owned: |&'a [u8]| -> MaybeOwned<'a>) -> MaybeOwned<'a> {
        match *self {
            UTF8(ref s) => s.as_slice().into_maybe_owned(),
            Bytes(ref v) => as_maybe_owned(v.as_slice()),
        }
    }

    pub fn as_maybe_owned<'a>(&'a self) -> MaybeOwned<'a> {
        self.map_as_maybe_owned(str::from_utf8_lossy)
    }

    pub fn into_str(self) -> Result<String, MaybeUTF8> {
        match self {
            UTF8(s) => Ok(s),
            Bytes(v) => match String::from_utf8(v) {
                Ok(s) => Ok(s),
                Err(v) => Err(Bytes(v)),
            },
        }
    }

    pub fn map_into_str(self, into_str: |Vec<u8>| -> String) -> String {
        match self {
            UTF8(s) => s,
            Bytes(v) => into_str(v),
        }
    }

    pub fn into_str_lossy(self) -> String {
        self.map_into_str(|v| match str::from_utf8_lossy(v.as_slice()) {
            // `v` is definitely UTF-8, so do not make a copy!
            str::Slice(_) => unsafe {str::raw::from_utf8_owned(v)},
            str::Owned(s) => s,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            UTF8(s) => s.into_bytes(),
            Bytes(v) => v,
        }
    }
}

impl PartialEq for MaybeUTF8 {
    fn eq(&self, other: &MaybeUTF8) -> bool {
        self.as_bytes().eq(&other.as_bytes())
    }
}

impl TotalEq for MaybeUTF8 {
}

impl PartialOrd for MaybeUTF8 {
    fn lt(&self, other: &MaybeUTF8) -> bool {
        self.as_bytes().lt(&other.as_bytes())
    }
}

impl TotalOrd for MaybeUTF8 {
    fn cmp(&self, other: &MaybeUTF8) -> Ordering {
        self.as_bytes().cmp(&other.as_bytes())
    }
}

impl BytesContainer for MaybeUTF8 {
    fn container_as_bytes<'a>(&'a self) -> &'a [u8] {
        self.as_bytes()
    }

    fn container_into_owned_bytes(self) -> Vec<u8> {
        self.into_bytes()
    }

    fn container_as_str<'a>(&'a self) -> Option<&'a str> {
        self.as_str()
    }
}

impl<T:BytesContainer> Equiv<T> for MaybeUTF8 {
    fn equiv(&self, other: &T) -> bool {
        self.as_bytes() == other.container_as_bytes()
    }
}

impl Container for MaybeUTF8 {
    fn len(&self) -> uint {
        self.as_bytes().len()
    }
}

impl Mutable for MaybeUTF8 {
    fn clear(&mut self) {
        match *self {
            UTF8(ref mut s) => s.clear(),
            Bytes(ref mut v) => v.clear(),
        }
    }
}

// a workaround for multiple `FromIterator` implementations with differing type params
trait MaybeUTF8FromIterator {
    fn maybe_utf8_from_iter<I:Iterator<Self>>(iterator: I) -> MaybeUTF8;
}

impl MaybeUTF8FromIterator for char {
    fn maybe_utf8_from_iter<I:Iterator<char>>(iterator: I) -> MaybeUTF8 {
        MaybeUTF8::from_str(FromIterator::from_iter(iterator))
    }
}

impl MaybeUTF8FromIterator for u8 {
    fn maybe_utf8_from_iter<I:Iterator<u8>>(iterator: I) -> MaybeUTF8 {
        MaybeUTF8::from_bytes(FromIterator::from_iter(iterator))
    }
}

impl<T:MaybeUTF8FromIterator> FromIterator<T> for MaybeUTF8 {
    fn from_iter<I:Iterator<T>>(iterator: I) -> MaybeUTF8 {
        MaybeUTF8FromIterator::maybe_utf8_from_iter(iterator)
    }
}

impl Default for MaybeUTF8 {
    fn default() -> MaybeUTF8 {
        MaybeUTF8::new()
    }
}

impl fmt::Show for MaybeUTF8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write(self.as_bytes())
    }
}

