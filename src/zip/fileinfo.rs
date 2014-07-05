use format;
use maybe_utf8::MaybeUTF8;

#[deriving(PartialEq, Show, Clone)]
pub enum CompressionMethod {
    Store=0,
    Deflate=8,
    Unknown
}

impl CompressionMethod {
    pub fn from_u16(x: u16) -> CompressionMethod {
        let u = x as uint;
        if      u == (Store   as uint) { Store }
        else if u == (Deflate as uint) { Deflate }
        else                           { Unknown }
    }
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

