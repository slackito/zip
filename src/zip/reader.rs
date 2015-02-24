
use std::old_io::File;
use std::old_io::{Reader, Writer, Seek, SeekSet, SeekEnd};
use std::iter::range_inclusive;
use std::old_path::BytesContainer;
use error::{ZipResult,ZipError};
use format::LocalFileHeader;
use maybe_utf8::MaybeUTF8;
use flate;
use crc32;
use format;
use fileinfo::{CompressionMethod, FileInfo};

pub struct ZipReader<R> {
    reader: R,
    end_record: format::EndOfCentralDirectoryRecord,
}

pub struct RawFiles<'a, R:'a> {
    zip_reader: &'a mut ZipReader<R>,
    current_entry: u16,
    current_offset: u64,
}

impl<'a, R: Reader+Seek> Iterator for RawFiles<'a, R> {
    type Item = Result<FileInfo, ZipError>;
    fn next(&mut self) -> Option<Result<FileInfo, ZipError>> {
        if self.current_entry < self.zip_reader.end_record.total_entry_count {
            match self.zip_reader.reader.seek(self.current_offset as i64, SeekSet) {
                Ok(()) => {}
                Err(err) => { return Some(Err(ZipError::IoError(err))); }
            }
            let h = match format::CentralDirectoryHeader::read(&mut self.zip_reader.reader) {
                Ok(h) => h,
                Err(err) => { return Some(Err(err)); }
            };
            let info = FileInfo::from_cdh(&h);
            self.current_entry += 1;
            self.current_offset += h.total_size() as u64;
            Some(Ok(info))
        } else {
            None
        }
    }
}

pub struct Files<'a, R:'a> {
    base: RawFiles<'a, R>,
}

impl<'a, R: Reader+Seek> Iterator for Files<'a, R> {
    type Item = FileInfo;
    fn next(&mut self) -> Option<FileInfo> { self.base.next().map(|i| i.ok().unwrap()) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.base.size_hint() }
}

pub struct FileNames<'a, R:'a> {
    base: RawFiles<'a, R>,
}

impl<'a, R: Reader+Seek> Iterator for FileNames<'a, R> {
    type Item = MaybeUTF8;
    fn next(&mut self) -> Option<MaybeUTF8> { self.base.next().map(|i| i.ok().unwrap().name) }
    fn size_hint(&self) -> (usize, Option<usize>) { self.base.size_hint() }
}

impl ZipReader<File> {
    pub fn open(path: &Path) -> Result<ZipReader<File>, ZipError> {
        ZipReader::new(try_io!(File::open(path)))
    }
}

impl<R:Reader+Seek> ZipReader<R> {
    pub fn new(reader: R) -> Result<ZipReader<R>, ZipError> {
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
                let e = try!(format::EndOfCentralDirectoryRecord::read(&mut r));
                Ok(ZipReader {reader: r, end_record: e})
            },
            None => Err(ZipError::NotAZipFile)
        }
    }

    pub fn files_raw<'a>(&'a mut self) -> RawFiles<'a, R> {
        let cdr_offset = self.end_record.central_directory_offset;
        RawFiles {
            zip_reader: self,
            current_entry: 0,
            current_offset: cdr_offset as u64
        }
    }

    pub fn files<'a>(&'a mut self) -> Files<'a, R> {
        Files { base: self.files_raw() }
    }

    pub fn file_names<'a>(&'a mut self) -> FileNames<'a, R> {
        FileNames { base: self.files_raw() }
    }

    pub fn info<T: BytesContainer>(&mut self, name: T) -> Result<FileInfo, ZipError> {
        for i in self.files() {
            if i.name == name.container_as_bytes() {
                return Ok(i);
            }
        }
        Err(ZipError::FileNotFoundInArchive)
    }

    fn read_header(&mut self, f: &FileInfo) -> ZipResult<LocalFileHeader> {
        try_io!(self.reader.seek(f.local_file_header_offset as i64, SeekSet));
        format::LocalFileHeader::read(&mut self.reader)
    }

    pub fn read(&mut self, f: &FileInfo) -> Result<Vec<u8>, ZipError> {
        let header = try!(self.read_header(f));
        header.print();

                
        let file_pos = f.local_file_header_offset as i64 + header.total_size() as i64;
        let file_len = header.compressed_size as usize;        
        self.extract_block(file_pos, file_len, header.compression_method, header.crc32)
    }
    
    fn extract_block(&mut self, pos: i64, len: usize, method: u16, crc32: u32) -> Result<Vec<u8>, ZipError> {
        try_io!(self.reader.seek(pos, SeekSet));
        let compressed = try_io!(self.reader.read_exact(len));
        match CompressionMethod::from_u16(method) {
                CompressionMethod::Store   => Ok(compressed),
                CompressionMethod::Deflate => self.decompress(compressed, crc32),
                _ => panic!("Usupported CompressionMethod")
        }
    }

    fn decompress(&mut self, data: Vec<u8>, crc32: u32) -> Result<Vec<u8>, ZipError> { 
        match flate::inflate_bytes(&data[]){ 
            Some(ok) => if crc32::crc32(&ok) == crc32 { Ok(ok.to_vec()) } else { Err(ZipError::CrcError) },
            None => return Err(ZipError::DecompressionFailure),
        }
    }

    // when we make read return a Reader, we will be able to loop here and copy
    // blocks of a fixed size from Reader to Writer
    pub fn extract<T:Writer>(&mut self, f: &FileInfo, writer: &mut T) -> Result<(), ZipError> {

        match self.read(f) {
            Ok(bytes) => { try_io!(writer.write_all(&bytes[])); Ok(()) },
            Err(x) => Err(x)
        }
    }

}

