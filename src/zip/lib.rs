#[desc="A simple rust library for reading and writing ZIP files"]
#[license="MIT"]

extern mod extra;

use std::rt::io::{Reader, Writer, Seek, SeekSet, SeekEnd};
use std::rt::io::extensions::{ReaderUtil, ReaderByteConversions};
use std::iter::range_inclusive;
use std::str; // TODO: look into std::ascii to see if it's a better fit
use extra::flate;

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
    file_name:                 ~str,
    extra_field:               ~[u8]
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
            file_name: ~"",
            extra_field: ~[]
        }
    }

    // reads a LocalFileHeader from the current position of the reader r
    fn read<T:Reader>(r: &mut T) -> Result<~LocalFileHeader, ~str> {
        let mut h = LocalFileHeader::new();

        h.signature = r.read_le_u32_();
        if h.signature != 0x04034b50 {
            return Err(~"invalid signature");
        }

        h.version_needed_to_extract = r.read_le_u16_();
        h.general_purpose_bit_flag = r.read_le_u16_();
        h.compression_method = r.read_le_u16_();
        h.last_modified_time = r.read_le_u16_();
        h.last_modified_date = r.read_le_u16_();
        h.crc32 = r.read_le_u32_();
        h.compressed_size = r.read_le_u32_();
        h.uncompressed_size = r.read_le_u32_();
        h.file_name_length = r.read_le_u16_();
        h.extra_field_length = r.read_le_u16_();
        h.file_name = str::from_utf8(r.read_bytes(h.file_name_length as uint));
        h.extra_field = r.read_bytes(h.extra_field_length as uint);

        // check for some things we don't support (yet?)
        assert!(!h.is_encrypted());
        assert!(!h.is_compressed_patched_data());
        assert!(!h.has_data_descriptor());
        assert!(!h.uses_strong_encryption());
        assert!(!h.uses_masking());

        Ok(~h)
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
    file_name: ~str,
    extra_field: ~[u8],
    file_comment: ~str,
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
            file_name: ~"",
            extra_field: ~[],
            file_comment: ~"",
        }
    }

    // reads a CentralDirectoryHeader from the current position of the reader r
    fn read<T:Reader>(r: &mut T) -> Result<~CentralDirectoryHeader, ~str> {
        let mut h = CentralDirectoryHeader::new();

        h.signature = r.read_le_u32_();
        if h.signature != 0x02014b50 {
            return Err(~"invalid signature");
        }

        h.version_made_by = r.read_le_u16_();
        h.version_needed_to_extract = r.read_le_u16_();
        h.general_purpose_bit_flag = r.read_le_u16_();
        h.compression_method = r.read_le_u16_();
        h.last_modified_time = r.read_le_u16_();
        h.last_modified_date = r.read_le_u16_();
        h.crc32 = r.read_le_u32_();
        h.compressed_size = r.read_le_u32_();
        h.uncompressed_size = r.read_le_u32_();
        h.file_name_length = r.read_le_u16_();
        h.extra_field_length = r.read_le_u16_();
        h.file_comment_length = r.read_le_u16_();
        h.disk_number_start = r.read_le_u16_();
        h.internal_file_attributes = r.read_le_u16_();
        h.external_file_attributes = r.read_le_u32_();
        h.relative_offset_of_local_header = r.read_le_u32_();
        h.file_name = str::from_utf8(r.read_bytes(h.file_name_length as uint));
        h.extra_field = r.read_bytes(h.extra_field_length as uint);
        h.file_comment = str::from_utf8(r.read_bytes(h.file_comment_length as uint));

        // check for some things we don't support (yet?)
        // TODO

        Ok(~h)
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
    data: ~[u8]
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
    comment: ~str
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
            comment: ~""
        }
    }

    fn read<T:Reader>(r: &mut T) -> Result<~EndOfCentralDirectoryRecord, ~str> {
        let mut h = EndOfCentralDirectoryRecord::new();

        h.signature = r.read_le_u32_();
        
        if h.signature != 0x06054b50 {
            return Err(~"invalid signature");
        }

        h.disk_number = r.read_le_u16_();
        h.disk_number_with_start_of_central_directory = r.read_le_u16_();
        h.entry_count_this_disk = r.read_le_u16_();
        h.total_entry_count = r.read_le_u16_();
        h.central_directory_size = r.read_le_u32_();
        h.central_directory_offset = r.read_le_u32_();
        h.comment_length = r.read_le_u16_();
        h.comment = str::from_utf8(r.read_bytes(h.comment_length as uint));

        // check for some things we don't support (yet?)
        // TODO

        Ok(~h)
    }

}




// ---- PUBLIC API STUFF ----
#[deriving(Eq,ToStr)]
pub enum ZipError {
    NotAZipFile,
    CrcError,
    FileNotFoundInArchive
}


#[deriving(Eq,ToStr,Clone)]
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
    name:               ~str,
    compression_method: CompressionMethod,
    last_modified_time: (int, int, int), // (hour, minute, second)
    last_modified_date: (int, int, int), // (year, month, day)
    crc32:              u32,
    compressed_size:    u32,
    uncompressed_size:  u32,
    is_encrypted:       bool,

    local_file_header_offset: u32,
}

pub struct ZipReader<T> {
    reader: T,
    end_record: ~EndOfCentralDirectoryRecord,
}

pub struct ZipReaderIterator<'self, T> {
    zip_reader: &'self mut ZipReader<T>,
    current_entry: u16,
    current_offset: u64,
}

impl<'self, T:Reader+Seek> Iterator<FileInfo> for ZipReaderIterator<'self, T> {
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
        r.seek(0, SeekEnd);
        let file_size = r.tell();
        let mut end_record_offset : Option<u64> = None;
        for i in range_inclusive(4, file_size) {
            let offset = file_size - i;
            r.seek(offset as i64, SeekSet);
            
            let sig = r.read_le_u32_();

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

    pub fn infolist(&mut self) -> ~[FileInfo] {
        let mut result : ~[FileInfo] = ~[];
        for info in self.iter() {
            result.push(info);
        }
        result
    }

    pub fn namelist(&mut self) -> ~[~str] {
        let mut result : ~[~str] = ~[];
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
    pub fn read(&mut self, f: &FileInfo) -> Result<~[u8], ZipError> {
        self.reader.seek(f.local_file_header_offset as i64, SeekSet);
        let h = LocalFileHeader::read(&mut self.reader).unwrap();
        let file_offset = f.local_file_header_offset as i64 + h.total_size() as i64;

        let result = 
            match u16_to_CompressionMethod(h.compression_method) {
                Store => self.read_stored_file(file_offset, h.uncompressed_size),
                Deflate => self.read_deflated_file(file_offset, h.compressed_size, h.uncompressed_size),
                _ => fail!()
            };

        // Check the CRC32 of the result against the one stored in the header
        let crc = crc32::crc32(result);

        if crc == h.crc32 { Ok(result) }
        else { Err(CrcError) }
    }

    fn read_stored_file(&mut self, pos: i64, uncompressed_size: u32) -> ~[u8] {
        self.reader.seek(pos, SeekSet);
        self.reader.read_bytes(uncompressed_size as uint)
    }

    fn read_deflated_file(&mut self, pos: i64, compressed_size: u32, uncompressed_size: u32) -> ~[u8] {
        self.reader.seek(pos, SeekSet);
        let compressed_bytes = self.reader.read_bytes(compressed_size as uint);
        let uncompressed_bytes = flate::inflate_bytes(compressed_bytes);
        assert!(uncompressed_bytes.len() as u32 == uncompressed_size);
        uncompressed_bytes
    }

    // when we make read return a Reader, we will be able to loop here and copy
    // blocks of a fixed size from Reader to Writer
    pub fn extract<T:Writer>(&mut self, f: &FileInfo, writer: &mut T) -> Result<(), ZipError> {
        match self.read(f) {
            Ok(bytes) => {writer.write(bytes); Ok(())},
            Err(x) => Err(x)
        }
    }

}



