#![crate_id = "zip"]
#![crate_type = "lib"]

#![desc="A simple rust library for reading and writing ZIP files"]
#![license="MIT"]

extern crate flate;

use std::io::{Reader, Writer, Seek, SeekSet, SeekEnd};
use std::io::{IoResult, IoError, InvalidInput};
use std::iter::range_inclusive;
use std::str; // TODO: look into std::ascii to see if it's a better fit

mod crc32;

// utility functions

// (year, month, day)
fn decode_msdos_date(date: u16) -> (int, int, int) {
    let d = date as int;
    ((d >> 9) + 1980, (d>>5) & 0xF, d & 0x1F)
}
// (hour, minute, second)
fn decode_msdos_time(time: u16) -> (int, int, int) {
    let t = time as int;
    ((t >> 11) & 0x1F, (t >> 5) & 0x3F, t & 0x1F)
}

fn invalid_signature<T>() -> IoResult<T> {
    Err(IoError { kind: InvalidInput, desc: "invalid signature", detail: None })
}

//  http://www.pkware.com/documents/casestudies/APPNOTE.TXT
//
//  4.3.6 Overall .ZIP file format:
//
//  [local file header 1]
//  [encryption header 1]
//  [file data 1]
//  [data descriptor 1]
//  . 
//  .
//  .
//  [local file header n]
//  [encryption header n]
//  [file data n]
//  [data descriptor n]
//  [archive decryption header] 
//  [archive extra data record] 
//  [central directory header 1]
//  .
//  .
//  .
//  [central directory header n]
//  [zip64 end of central directory record]
//  [zip64 end of central directory locator] 
//  [end of central directory record]


// ==== LOCAL FILE HEADER ====

struct LocalFileHeader {
    signature:                 u32, // 0x04034b50
    version_needed_to_extract: u16,
    general_purpose_bit_flag:  u16,
    compression_method:        u16,
    last_modified_time:        u16,
    last_modified_date:        u16,
    crc32:                     u32,
    compressed_size:           u32,
    uncompressed_size:         u32,
    file_name_length:          u16,
    extra_field_length:        u16,
    file_name:                 String,
    extra_field:               Vec<u8>
}

impl LocalFileHeader {
    // -- header property getters
    
    // see section 4.4.4 of APPNOTE.TXT for more info about these flags
    fn is_encrypted(&self) -> bool               { (self.general_purpose_bit_flag &    1) != 0 } 
    fn has_data_descriptor(&self) -> bool        { (self.general_purpose_bit_flag &    8) != 0 }
    fn is_compressed_patched_data(&self) -> bool { (self.general_purpose_bit_flag &   32) != 0 }
    fn uses_strong_encryption(&self) -> bool     { (self.general_purpose_bit_flag &   64) != 0 }
    fn has_UTF8_name(&self) -> bool              { (self.general_purpose_bit_flag & 2048) != 0 }
    fn uses_masking(&self) -> bool               { (self.general_purpose_bit_flag & 8192) != 0 }

    fn total_size(&self) -> int {
        let local_file_header_fixed_size = 30;
        local_file_header_fixed_size + (self.file_name_length as int) + (self.extra_field_length as int) 
    }

    // -- constructors
    fn new() -> LocalFileHeader {
        LocalFileHeader{
            signature: 0,
            version_needed_to_extract: 0,
            general_purpose_bit_flag: 0,
            compression_method: 0,
            last_modified_time: 0,
            last_modified_date: 0,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            file_name_length: 0,
            extra_field_length: 0,
            file_name: String::new(),
            extra_field: Vec::new()
        }
    }

    // reads a LocalFileHeader from the current position of the reader r
    fn read<T:Reader>(r: &mut T) -> IoResult<LocalFileHeader> {
        let mut h = LocalFileHeader::new();

        h.signature = try!(r.read_le_u32());
        if h.signature != 0x04034b50 {
            return invalid_signature();
        }

        h.version_needed_to_extract = try!(r.read_le_u16());
        h.general_purpose_bit_flag = try!(r.read_le_u16());
        h.compression_method = try!(r.read_le_u16());
        h.last_modified_time = try!(r.read_le_u16());
        h.last_modified_date = try!(r.read_le_u16());
        h.crc32 = try!(r.read_le_u32());
        h.compressed_size = try!(r.read_le_u32());
        h.uncompressed_size = try!(r.read_le_u32());
        h.file_name_length = try!(r.read_le_u16());
        h.extra_field_length = try!(r.read_le_u16());
        h.file_name = str::from_utf8_owned(try!(r.read_exact(h.file_name_length as uint))).unwrap();
        h.extra_field = try!(r.read_exact(h.extra_field_length as uint));

        // check for some things we don't support (yet?)
        assert!(!h.is_encrypted());
        assert!(!h.is_compressed_patched_data());
        assert!(!h.has_data_descriptor());
        assert!(!h.uses_strong_encryption());
        assert!(!h.uses_masking());

        Ok(h)
    }

    fn write<T:Writer>(&self, w: &mut T) -> IoResult<()> {
        try!(w.write_le_u32(self.signature));
        try!(w.write_le_u16(self.version_needed_to_extract));
        try!(w.write_le_u16(self.general_purpose_bit_flag));
        try!(w.write_le_u16(self.compression_method));
        try!(w.write_le_u16(self.last_modified_time));
        try!(w.write_le_u16(self.last_modified_date));
        try!(w.write_le_u32(self.crc32));
        try!(w.write_le_u32(self.compressed_size));
        try!(w.write_le_u32(self.uncompressed_size));
        try!(w.write_le_u16(self.file_name_length));
        try!(w.write_le_u16(self.extra_field_length));
        try!(w.write(self.file_name.as_bytes()));
        try!(w.write(self.extra_field.as_slice()));
        Ok(())
    }

    // for debug purposes
    fn print(&self) {
        println!("signature: {:#08x}", self.signature);
        println!("version_needed_to_extract: {:#04x}", self.version_needed_to_extract);
        println!("general_purpose_bit_flag: {:#04x}", self.general_purpose_bit_flag);
        println!("compression_method: {:#04x}", self.compression_method);
        println!("last_modified_time: {:?}", decode_msdos_time(self.last_modified_time));
        println!("last_modified_date: {:?}", decode_msdos_date(self.last_modified_date)); 
        println!("crc32: {:#08x}", self.crc32);
        println!("compressed_size: {}", self.compressed_size);
        println!("uncompressed_size: {}", self.uncompressed_size);
        println!("file_name_length: {}", self.file_name_length);
        println!("extra_field_length: {}", self.extra_field_length);
        println!("file_name: {}", self.file_name);
        println!("extra_field: {:?}", self.extra_field);

        println!("FLAGS: ");
        println!("  is encrypted: {}", self.is_encrypted());
        println!("  has data descriptor: {}", self.has_data_descriptor());
        println!("  is compressed patched data: {}", self.is_compressed_patched_data());
        println!("  uses strong encryption: {}", self.uses_strong_encryption());
        println!("  has UFT8 name: {}", self.has_UTF8_name());
        println!("  uses masking: {}", self.uses_masking());
    }
}

// TODO: Add support for data descriptor section after the file contents (typically used when the zip file
// writer doesn't know the file size beforehand, because it's receiving a stream of data or something)
struct DataDescriptor {
    signature: u32, // optional: 0x08074b50
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
}

// ==== CENTRAL DIRECTORY HEADER ====

struct CentralDirectoryHeader {
    signature: u32, // 0x02014b50
    version_made_by: u16,
    version_needed_to_extract: u16,
    general_purpose_bit_flag: u16,
    compression_method: u16,
    last_modified_time: u16,
    last_modified_date: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    file_name_length: u16,
    extra_field_length: u16,
    file_comment_length: u16,
    disk_number_start: u16,
    internal_file_attributes: u16,
    external_file_attributes: u32,
    relative_offset_of_local_header: u32,
    file_name: String,
    extra_field: Vec<u8>,
    file_comment: String,
}

impl CentralDirectoryHeader {
    fn is_encrypted(&self) -> bool               { (self.general_purpose_bit_flag &    1) != 0 } 
    fn has_data_descriptor(&self) -> bool        { (self.general_purpose_bit_flag &    8) != 0 }
    fn is_compressed_patched_data(&self) -> bool { (self.general_purpose_bit_flag &   32) != 0 }
    fn uses_strong_encryption(&self) -> bool     { (self.general_purpose_bit_flag &   64) != 0 }
    fn has_UTF8_name(&self) -> bool              { (self.general_purpose_bit_flag & 2048) != 0 }
    fn uses_masking(&self) -> bool               { (self.general_purpose_bit_flag & 8192) != 0 }

    fn total_size(&self) -> int { 
        let central_directory_header_fixed_size = 46;
        central_directory_header_fixed_size
            + (self.file_name_length as int)
            + (self.extra_field_length as int)
            + (self.file_comment_length as int)
    }


    fn new() -> CentralDirectoryHeader {
        CentralDirectoryHeader {
            signature: 0,
            version_made_by: 0,
            version_needed_to_extract: 0,
            general_purpose_bit_flag: 0,
            compression_method: 0,
            last_modified_time: 0,
            last_modified_date: 0,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            file_name_length: 0,
            extra_field_length: 0,
            file_comment_length: 0,
            disk_number_start: 0,
            internal_file_attributes: 0,
            external_file_attributes: 0,
            relative_offset_of_local_header: 0,
            file_name: String::new(),
            extra_field: Vec::new(),
            file_comment: String::new(),
        }
    }

    // reads a CentralDirectoryHeader from the current position of the reader r
    fn read<T:Reader>(r: &mut T) -> IoResult<CentralDirectoryHeader> {
        let mut h = CentralDirectoryHeader::new();

        h.signature = try!(r.read_le_u32());
        if h.signature != 0x02014b50 {
            return invalid_signature();
        }

        h.version_made_by = try!(r.read_le_u16());
        h.version_needed_to_extract = try!(r.read_le_u16());
        h.general_purpose_bit_flag = try!(r.read_le_u16());
        h.compression_method = try!(r.read_le_u16());
        h.last_modified_time = try!(r.read_le_u16());
        h.last_modified_date = try!(r.read_le_u16());
        h.crc32 = try!(r.read_le_u32());
        h.compressed_size = try!(r.read_le_u32());
        h.uncompressed_size = try!(r.read_le_u32());
        h.file_name_length = try!(r.read_le_u16());
        h.extra_field_length = try!(r.read_le_u16());
        h.file_comment_length = try!(r.read_le_u16());
        h.disk_number_start = try!(r.read_le_u16());
        h.internal_file_attributes = try!(r.read_le_u16());
        h.external_file_attributes = try!(r.read_le_u32());
        h.relative_offset_of_local_header = try!(r.read_le_u32());
        h.file_name = str::from_utf8_owned(try!(r.read_exact(h.file_name_length as uint))).unwrap();
        h.extra_field = try!(r.read_exact(h.extra_field_length as uint));
        h.file_comment = str::from_utf8_owned(try!(r.read_exact(h.file_comment_length as uint))).unwrap();

        // check for some things we don't support (yet?)
        // TODO

        Ok(h)
    }

    fn write<T:Writer>(&self, w: &mut T) -> IoResult<()> {
        try!(w.write_le_u32(self.signature));
        try!(w.write_le_u16(self.version_made_by));
        try!(w.write_le_u16(self.version_needed_to_extract));
        try!(w.write_le_u16(self.general_purpose_bit_flag));
        try!(w.write_le_u16(self.compression_method));
        try!(w.write_le_u16(self.last_modified_time));
        try!(w.write_le_u16(self.last_modified_date));
        try!(w.write_le_u32(self.crc32));
        try!(w.write_le_u32(self.compressed_size));
        try!(w.write_le_u32(self.uncompressed_size));
        try!(w.write_le_u16(self.file_name_length));
        try!(w.write_le_u16(self.extra_field_length));
        try!(w.write_le_u16(self.file_comment_length));
        try!(w.write_le_u16(self.disk_number_start));
        try!(w.write_le_u16(self.internal_file_attributes));
        try!(w.write_le_u32(self.external_file_attributes));
        try!(w.write_le_u32(self.relative_offset_of_local_header));
        try!(w.write(self.file_name.as_bytes()));
        try!(w.write(self.extra_field.as_slice()));
        try!(w.write(self.file_comment.as_bytes()));
        Ok(())
    }


    // fills a FileInfo struct with the file properties, for users of the external API to see
    fn to_file_info(&self) -> FileInfo {
        let method : CompressionMethod = 
            if self.compression_method == 0 { Store }
            else if self.compression_method == 8 { Deflate }
            else { fail!() };
        FileInfo {
            name:               self.file_name.clone(),
            compression_method: method,
            last_modified_time: decode_msdos_time(self.last_modified_time),
            last_modified_date: decode_msdos_date(self.last_modified_date),
            crc32:              self.crc32,
            compressed_size:    self.compressed_size,
            uncompressed_size:  self.uncompressed_size,
            local_file_header_offset: self.relative_offset_of_local_header,
            is_encrypted:       self.is_encrypted(),
        }
    }

}

struct CentralDirectoryDigitalSignature {
    signature: u32, // 0x05054b50
    data_size: u16,
    data: Vec<u8>
}


// ==== END OF CENTRAL DIRECTORY RECORD ====

struct EndOfCentralDirectoryRecord {
    signature: u32, // 0x06054b50
    disk_number: u16,
    disk_number_with_start_of_central_directory: u16,
    entry_count_this_disk: u16,
    total_entry_count: u16,
    central_directory_size: u32,
    central_directory_offset: u32,
    comment_length: u16,
    comment: String
}

impl EndOfCentralDirectoryRecord {
    fn new() -> EndOfCentralDirectoryRecord {
        EndOfCentralDirectoryRecord {
            signature: 0,
            disk_number: 0,
            disk_number_with_start_of_central_directory: 0,
            entry_count_this_disk: 0,
            total_entry_count: 0,
            central_directory_size: 0,
            central_directory_offset: 0,
            comment_length: 0,
            comment: String::new()
        }
    }

    fn read<T:Reader>(r: &mut T) -> IoResult<EndOfCentralDirectoryRecord> {
        let mut h = EndOfCentralDirectoryRecord::new();

        h.signature = try!(r.read_le_u32());
        
        if h.signature != 0x06054b50 {
            return invalid_signature();
        }

        h.disk_number = try!(r.read_le_u16());
        h.disk_number_with_start_of_central_directory = try!(r.read_le_u16());
        h.entry_count_this_disk = try!(r.read_le_u16());
        h.total_entry_count = try!(r.read_le_u16());
        h.central_directory_size = try!(r.read_le_u32());
        h.central_directory_offset = try!(r.read_le_u32());
        h.comment_length = try!(r.read_le_u16());
        h.comment = str::from_utf8_owned(try!(r.read_exact(h.comment_length as uint))).unwrap();

        // check for some things we don't support (yet?)
        // TODO

        Ok(h)
    }

    fn write<T:Writer>(&self, w: &mut T) -> IoResult<()> {
        try!(w.write_le_u32(self.signature));
        try!(w.write_le_u16(self.disk_number));
        try!(w.write_le_u16(self.disk_number_with_start_of_central_directory));
        try!(w.write_le_u16(self.entry_count_this_disk));
        try!(w.write_le_u16(self.total_entry_count));
        try!(w.write_le_u32(self.central_directory_size));
        try!(w.write_le_u32(self.central_directory_offset));
        try!(w.write_le_u16(self.comment_length));
        try!(w.write(self.comment.as_bytes()));
        Ok(())
    }

}




// ---- PUBLIC API STUFF ----
#[deriving(Eq,Show)]
pub enum ZipError {
    IoError(IoError),
    NotAZipFile,
    CrcError,
    FileNotFoundInArchive
}

fn io_result_to_zip_result<T>(x: IoResult<T>) -> Result<T, ZipError> {
    match x {
        Ok(v) => Ok(v),
        Err(e) => Err(IoError(e)),
    }
}


#[deriving(Eq,Show,Clone)]
pub enum CompressionMethod {
    Store=0,
    Deflate=8,
    Unknown
}

fn u16_to_CompressionMethod(x: u16) -> CompressionMethod {
    let u = x as uint;
    if      u == (Store   as uint) { Store }
    else if u == (Deflate as uint) { Deflate }
    else                           { Unknown }
}


#[deriving(Clone)]
pub struct FileInfo {
    pub name:               String,
    pub compression_method: CompressionMethod,
    pub last_modified_time: (int, int, int), // (hour, minute, second)
    pub last_modified_date: (int, int, int), // (year, month, day)
    pub crc32:              u32,
    pub compressed_size:    u32,
    pub uncompressed_size:  u32,
    pub is_encrypted:       bool,

    pub local_file_header_offset: u32,
}

pub struct ZipReader<T> {
    reader: T,
    end_record: EndOfCentralDirectoryRecord,
}

pub struct ZipReaderIterator<'a, T> {
    zip_reader: &'a mut ZipReader<T>,
    current_entry: u16,
    current_offset: u64,
}

impl<'a, T:Reader+Seek> Iterator<FileInfo> for ZipReaderIterator<'a, T> {
    fn next(&mut self) -> Option<FileInfo> {
        if (self.current_entry < self.zip_reader.end_record.total_entry_count) {
            self.zip_reader.reader.seek(self.current_offset as i64, SeekSet);
            let h = CentralDirectoryHeader::read(&mut self.zip_reader.reader).unwrap();
            let info = h.to_file_info();
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
        try!(io_result_to_zip_result(r.seek(0, SeekEnd)));
        let file_size = try!(io_result_to_zip_result(r.tell()));
        let mut end_record_offset : Option<u64> = None;
        for i in range_inclusive(4, file_size) {
            let offset = file_size - i;
            try!(io_result_to_zip_result(r.seek(offset as i64, SeekSet)));
            
            let sig = try!(io_result_to_zip_result(r.read_le_u32()));

            // TODO: check for false positives here
            if (sig == 0x06054b50) {
                end_record_offset = Some(offset);
                break;
            }
            
        }

        match end_record_offset {
            Some(offset) => {
                r.seek(offset as i64, SeekSet);
                let e = EndOfCentralDirectoryRecord::read(&mut r).unwrap();
                Ok(ZipReader {reader: r, end_record: e})
            },
            None => Err(NotAZipFile)
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

    pub fn namelist(&mut self) -> Vec<String> {
        let mut result = Vec::new();
        for info in self.iter() {
            result.push(info.name.clone());
        }
        result
    }

    pub fn get_file_info(&mut self, name: &str) -> Result<FileInfo, ZipError> {
        for i in self.iter() {
            if name.equiv(&i.name) {
                return Ok(i);
            }
        }
        Err(FileNotFoundInArchive)
    }

    // TODO: Create a Reader for the cases when you don't want to decompress the whole file
    pub fn read(&mut self, f: &FileInfo) -> Result<Vec<u8>, ZipError> {
        self.reader.seek(f.local_file_header_offset as i64, SeekSet);
        let h = LocalFileHeader::read(&mut self.reader).unwrap();
        let file_offset = f.local_file_header_offset as i64 + h.total_size() as i64;

        let result = 
            match u16_to_CompressionMethod(h.compression_method) {
                Store => self.read_stored_file(file_offset, h.uncompressed_size),
                Deflate => self.read_deflated_file(file_offset, h.compressed_size, h.uncompressed_size),
                _ => fail!()
            };
        let result = try!(io_result_to_zip_result(result));

        // Check the CRC32 of the result against the one stored in the header
        let crc = crc32::crc32(result.as_slice());

        if crc == h.crc32 { Ok(result) }
        else { Err(CrcError) }
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
            Ok(bytes) => {writer.write(bytes.as_slice()); Ok(())},
            Err(x) => Err(x)
        }
    }

}



