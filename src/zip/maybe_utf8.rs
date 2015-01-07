//! Byte container optionally encoded as UTF-8.

use std::{str, char, fmt};
use std::borrow::{IntoCow, Cow};
use std::string::CowString;
use std::default::Default;
use std::path::BytesContainer;
use std::cmp::Ordering;
use std::iter::FromIterator;

#[derive(Clone)]
pub enum MaybeUTF8 {
    UTF8(String),
    Bytes(Vec<u8>),
}

impl MaybeUTF8 {
    pub fn new() -> MaybeUTF8 {
        MaybeUTF8::UTF8(String::new())
    }

    pub fn from_str(s: String) -> MaybeUTF8 {
        MaybeUTF8::UTF8(s)
    }

    pub fn from_bytes(v: Vec<u8>) -> MaybeUTF8 {
        MaybeUTF8::Bytes(v)
    }

    pub fn as_bytes<'a>(&'a self) -> &'a [u8] {
        match *self {
            MaybeUTF8::UTF8(ref s) => s.as_bytes(),
            MaybeUTF8::Bytes(ref v) => v.as_slice(),
        }
    }

    pub fn as_str<'a>(&'a self) -> Option<&'a str> {
        match *self {
            MaybeUTF8::UTF8(ref s) => Some(s.as_slice()),
            MaybeUTF8::Bytes(ref v) => str::from_utf8(v.as_slice()).ok(),
        }
    }

    pub fn map_as_cow<'a, F>(&'a self, mut as_cow: F) -> CowString<'a>
            where F: FnMut(&'a [u8]) -> CowString<'a> {
        match *self {
            MaybeUTF8::UTF8(ref s) => s.as_slice().into_cow(),
            MaybeUTF8::Bytes(ref v) => as_cow(v.as_slice()),
        }
    }

    pub fn as_cow<'a>(&'a self) -> CowString<'a> {
        self.map_as_cow(String::from_utf8_lossy)
    }

    pub fn into_str(self) -> Result<String, MaybeUTF8> {
        match self {
            MaybeUTF8::UTF8(s) => Ok(s),
            MaybeUTF8::Bytes(v) => match String::from_utf8(v) {
                Ok(s) => Ok(s),
                Err(e) => Err(MaybeUTF8::Bytes(e.into_bytes())),
            },
        }
    }

    pub fn map_into_str<F>(self, mut into_str: F) -> String
            where F: FnMut(Vec<u8>) -> String {
        match self {
            MaybeUTF8::UTF8(s) => s,
            MaybeUTF8::Bytes(v) => into_str(v),
        }
    }

    pub fn into_str_lossy(self) -> String {
        self.map_into_str(|v| match String::from_utf8_lossy(v.as_slice()) {
            // `v` is definitely UTF-8, so do not make a copy!
            Cow::Borrowed(_) => unsafe {String::from_utf8_unchecked(v)},
            Cow::Owned(s) => s,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            MaybeUTF8::UTF8(s) => s.into_bytes(),
            MaybeUTF8::Bytes(v) => v,
        }
    }

    pub fn len(&self) -> uint {
        match *self {
            MaybeUTF8::UTF8(ref s) => s.len(),
            MaybeUTF8::Bytes(ref v) => v.len(),
        }
    }

    pub fn clear(&mut self) {
        match *self {
            MaybeUTF8::UTF8(ref mut s) => s.clear(),
            MaybeUTF8::Bytes(ref mut v) => v.clear(),
        }
    }
}

macro_rules! define_partial_eq_and_cmp {
    ($($lty:ty#$lmeth:ident, $rty:ty#$rmeth:ident;)*) => ($(
        impl<'a> PartialEq<$rty> for $lty {
            fn eq(&self, other: &$rty) -> bool { self.$lmeth().eq(other.$rmeth()) }
        }
        impl<'a> PartialOrd<$rty> for $lty {
            fn partial_cmp(&self, other: &$rty) -> Option<Ordering> {
                self.$lmeth().partial_cmp(other.$rmeth())
            }
        }
    )*)
}

define_partial_eq_and_cmp! {
    MaybeUTF8#as_bytes, MaybeUTF8#as_bytes;
    MaybeUTF8#as_bytes, &'a str#as_bytes;
    MaybeUTF8#as_bytes, &'a [u8]#as_slice;
}

impl Eq for MaybeUTF8 {
}

impl Ord for MaybeUTF8 {
    fn cmp(&self, other: &MaybeUTF8) -> Ordering {
        self.as_bytes().cmp(other.container_as_bytes())
    }
}

impl BytesContainer for MaybeUTF8 {
    fn container_as_bytes<'a>(&'a self) -> &'a [u8] {
        self.as_bytes()
    }

    fn container_as_str<'a>(&'a self) -> Option<&'a str> {
        self.as_str()
    }

    fn is_str(_: Option<&MaybeUTF8>) -> bool {
        false
    }
}

impl FromIterator<char> for MaybeUTF8 {
    fn from_iter<I: Iterator<Item=char>>(iterator: I) -> MaybeUTF8 {
        MaybeUTF8::from_str(FromIterator::from_iter(iterator))
    }
}

impl FromIterator<u8> for MaybeUTF8 {
    fn from_iter<I: Iterator<Item=u8>>(iterator: I) -> MaybeUTF8 {
        MaybeUTF8::from_bytes(FromIterator::from_iter(iterator))
    }
}

impl Default for MaybeUTF8 {
    fn default() -> MaybeUTF8 {
        MaybeUTF8::new()
    }
}

impl fmt::Show for MaybeUTF8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MaybeUTF8::UTF8(ref s) => fmt::Show::fmt(s, f),
            MaybeUTF8::Bytes(ref v) => {
                try!(write!(f, "b\""));
                for &c in v.iter() {
                    match c {
                        b'\t' => try!(write!(f, "\\t")),
                        b'\r' => try!(write!(f, "\\r")),
                        b'\n' => try!(write!(f, "\\n")),
                        b'\\' => try!(write!(f, "\\\\")),
                        b'\'' => try!(write!(f, "\\'")),
                        b'"'  => try!(write!(f, "\\\"")),
                        b'\x20' ... b'\x7e' => try!(write!(f, "{}", c as char)),
                        _ => try!(write!(f, "\\x{}{}",
                                         char::from_digit((c as uint) >> 4, 16).unwrap(),
                                         char::from_digit((c as uint) & 0xf, 16).unwrap()))
                    }
                }
                write!(f, "\"")
            }
        }
    }
}

