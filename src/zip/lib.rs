#![crate_id = "zip"]
#![crate_type = "lib"]

#![desc="A simple rust library for reading and writing ZIP files"]
#![license="MIT"]

#![feature(macro_rules)]

extern crate flate;

use std::io::{Reader, Writer, Seek, SeekSet, SeekEnd};
use std::io::{IoResult, IoError, InvalidInput};
use std::iter::range_inclusive;
use std::path::BytesContainer;
use error::ZipError;
use maybe_utf8::MaybeUTF8;

mod crc32;
pub mod maybe_utf8;
pub mod error;
pub mod format;

#[deriving(PartialEq,Show,Clone)]
pub enum CompressionMethod {
    Store=0,
    Deflate=8,
    Unknown
}

fn u16_to_compression_method(x: u16) -> CompressionMethod {
    let u = x as uint;
    if      u == (Store   as uint) { Store }
    else if u == (Deflate as uint) { Deflate }
    else                           { Unknown }
}


#[deriving(Clone)]
pub struct FileInfo {
    pub name:               MaybeUTF8,
    pub compression_method: CompressionMethod,
    // (year, month, day, hour, minute, second)
    pub last_modified_datetime: (uint, uint, uint, uint, uint, uint),
    pub crc32:              u32,
    pub compressed_size:    u32,
    pub uncompressed_size:  u32,
    pub is_encrypted:       bool,

    pub local_file_header_offset: u32,
}

impl FileInfo {
    // fills a FileInfo struct with the file properties, for users of the external API to see
    pub fn from_cdh(h: &format::CentralDirectoryHeader) -> FileInfo {
        let method : CompressionMethod = 
            if h.compression_method == 0 { Store }
            else if h.compression_method == 8 { Deflate }
            else { fail!() };
        FileInfo {
            name:               h.file_name.clone(),
            compression_method: method,
            last_modified_datetime: h.last_modified_datetime.to_tuple(),
            crc32:              h.crc32,
            compressed_size:    h.compressed_size,
            uncompressed_size:  h.uncompressed_size,
            local_file_header_offset: h.relative_offset_of_local_header,
            is_encrypted:       h.is_encrypted(),
        }
    }
}

pub struct ZipReader<T> {
    reader: T,
    end_record: format::EndOfCentralDirectoryRecord,
}

pub struct ZipReaderIterator<'a, T> {
    zip_reader: &'a mut ZipReader<T>,
    current_entry: u16,
    current_offset: u64,
}

impl<'a, T:Reader+Seek> Iterator<FileInfo> for ZipReaderIterator<'a, T> {
    fn next(&mut self) -> Option<FileInfo> {
        if self.current_entry < self.zip_reader.end_record.total_entry_count {
            self.zip_reader.reader.seek(self.current_offset as i64, SeekSet);
            let h = format::CentralDirectoryHeader::read(&mut self.zip_reader.reader).unwrap();
            let info = FileInfo::from_cdh(&h);
            self.current_entry += 1;
            self.current_offset += h.total_size() as u64;
            Some(info)
        } else {
            None
        }
    }
}

impl<T:Reader+Seek> ZipReader<T> {
    pub fn new(reader: T) -> Result<ZipReader<T>, ZipError> {
        // find the End of Central Directory record, looking backwards from the end of the file
        let mut r = reader;
        try_io!(r.seek(0, SeekEnd));
        let file_size = try_io!(r.tell());
        let mut end_record_offset : Option<u64> = None;
        for i in range_inclusive(4, file_size) {
            let offset = file_size - i;
            try_io!(r.seek(offset as i64, SeekSet));
            
            let sig = try_io!(r.read_le_u32());

            // TODO: check for false positives here
            if sig == format::EOCDR_SIGNATURE {
                end_record_offset = Some(offset);
                break;
            }
            
        }

        match end_record_offset {
            Some(offset) => {
                try_io!(r.seek(offset as i64, SeekSet));
                let e = format::EndOfCentralDirectoryRecord::read(&mut r).unwrap();
                Ok(ZipReader {reader: r, end_record: e})
            },
            None => Err(error::NotAZipFile)
        }
    }

    pub fn iter<'a>(&'a mut self) -> ZipReaderIterator<'a, T> {
        ZipReaderIterator {
            zip_reader: self,
            current_entry: 0,
            current_offset: self.end_record.central_directory_offset as u64
        }
    }

    pub fn infolist(&mut self) -> Vec<FileInfo> {
        let mut result = Vec::new();
        for info in self.iter() {
            result.push(info);
        }
        result
    }

    pub fn namelist(&mut self) -> Vec<MaybeUTF8> {
        let mut result = Vec::new();
        for info in self.iter() {
            result.push(info.name.clone());
        }
        result
    }

    pub fn get_file_info<T:BytesContainer>(&mut self, name: T) -> Result<FileInfo, ZipError> {
        for i in self.iter() {
            if i.name.equiv(&name) {
                return Ok(i);
            }
        }
        Err(error::FileNotFoundInArchive)
    }

    // TODO: Create a Reader for the cases when you don't want to decompress the whole file
    pub fn read(&mut self, f: &FileInfo) -> Result<Vec<u8>, ZipError> {
        try_io!(self.reader.seek(f.local_file_header_offset as i64, SeekSet));
        let h = format::LocalFileHeader::read(&mut self.reader).unwrap();
        let file_offset = f.local_file_header_offset as i64 + h.total_size() as i64;

        let result = 
            match u16_to_compression_method(h.compression_method) {
                Store => self.read_stored_file(file_offset, h.uncompressed_size),
                Deflate => self.read_deflated_file(file_offset, h.compressed_size, h.uncompressed_size),
                _ => fail!()
            };
        let result = try_io!(result);

        // Check the CRC32 of the result against the one stored in the header
        let crc = crc32::crc32(result.as_slice());

        if crc == h.crc32 { Ok(result) }
        else { Err(error::CrcError) }
    }

    fn read_stored_file(&mut self, pos: i64, uncompressed_size: u32) -> IoResult<Vec<u8>> {
        try!(self.reader.seek(pos, SeekSet));
        self.reader.read_exact(uncompressed_size as uint)
    }

    fn read_deflated_file(&mut self, pos: i64, compressed_size: u32, uncompressed_size: u32) -> IoResult<Vec<u8>> {
        try!(self.reader.seek(pos, SeekSet));
        let compressed_bytes = try!(self.reader.read_exact(compressed_size as uint));
        let uncompressed_bytes = match flate::inflate_bytes(compressed_bytes.as_slice()) {
            Some(bytes) => bytes,
            None => return Err(IoError { kind: InvalidInput, desc: "decompression failure", detail: None })
        };
        assert!(uncompressed_bytes.len() as u32 == uncompressed_size);
        // FIXME try not to copy the buffer, or switch to the incremental fashion
        Ok(Vec::from_slice(uncompressed_bytes.as_slice()))
    }

    // when we make read return a Reader, we will be able to loop here and copy
    // blocks of a fixed size from Reader to Writer
    pub fn extract<T:Writer>(&mut self, f: &FileInfo, writer: &mut T) -> Result<(), ZipError> {
        match self.read(f) {
            Ok(bytes) => { try_io!(writer.write(bytes.as_slice())); Ok(()) },
            Err(x) => Err(x)
        }
    }

}

