use std::{
    collections::HashMap,
    error::Error,
    fmt::Debug,
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};

///////////////////////////////////////////////

pub struct SqPackIndexFile(HashMap<u64, SqPackIndexTableEntry>);

impl SqPackIndexFile {
    pub fn from_file(file_path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let mut reader = BufReader::new(File::open(&file_path)?);
        let index1 = Self::from_reader1(&mut reader)?;

        let file_path = format!("{}2", file_path.as_ref().to_str().unwrap());
        let mut reader = BufReader::new(File::open(&file_path)?);
        let index2 = Self::from_reader2(&mut reader)?;

        let mut entries = index2.0;
        for entry in index1.0 {
            entries.insert(entry.0, entry.1);
        }

        Ok(Self(entries))
    }

    fn from_reader_common<R: ReadBytesExt + Seek>(reader: &mut R) -> Result<u32, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(0))?;
        let header = SqPackHeader::from_reader(reader)?;

        reader.seek(SeekFrom::Start(header.size.into()))?;
        let index_header = SqPackIndexHeader::from_reader(reader)?;

        reader.seek(SeekFrom::Start(index_header.index_data_offset.into()))?;
        Ok(index_header.index_data_size)
    }

    fn from_reader1<R: ReadBytesExt + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let data_size = Self::from_reader_common(reader)?;
        let num_entries = data_size / 16; // Two 64-bit values per table entry
        let mut entries = HashMap::new();
        for _ in 0..num_entries {
            let entry = SqPackIndexTableEntry::from_reader1(reader)?;
            entries.insert(entry.hash, entry);
        }

        Ok(Self(entries))
    }

    fn from_reader2<R: ReadBytesExt + Seek>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let data_size = Self::from_reader_common(reader)?;
        let num_entries = data_size / 8; // Two 32-bit values per table entry
        let mut entries = HashMap::new();
        for _ in 0..num_entries {
            let entry = SqPackIndexTableEntry::from_reader2(reader)?;
            entries.insert(entry.hash, entry);
        }

        Ok(Self(entries))
    }

    pub fn entry_from_path(&self, path: impl AsRef<str>) -> Option<&SqPackIndexTableEntry> {
        let resource = Resource::new(path)?;
        let hash1 = ((resource.directory.hash as u64) << 32) | (resource.file.hash as u64);
        let hash2 = resource.full_hash.hash as u64;
        self.0.get(&hash1).or_else(|| self.0.get(&hash2))
    }
}

///////////////////////////////////////////////

pub struct SqPackIndexTableEntry {
    pub hash: u64,
    pub data_file_id: u32,
    pub offset: u32,
}

impl SqPackIndexTableEntry {
    fn from_reader1<R: ReadBytesExt>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let hash = reader.read_u64::<LittleEndian>()?;
        let mut data = reader.read_u64::<LittleEndian>()?;

        data >>= 1;
        let data_file_id = (data & 0b111) as u32;
        let offset = ((data & !0x7) as u32) << 4;

        Ok(Self {
            hash,
            data_file_id,
            offset,
        })
    }

    fn from_reader2<R: ReadBytesExt>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let hash = reader.read_u32::<LittleEndian>()?;
        let mut data = reader.read_u32::<LittleEndian>()?;

        data >>= 1;
        let data_file_id = data & 0b1110;
        let offset = (data & !0x7) << 4;

        Ok(Self {
            hash: hash as u64,
            data_file_id,
            offset,
        })
    }
}

impl Debug for SqPackIndexTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqPackIndexTableEntry")
            .field("hash", &format!("{:016X}", self.hash))
            .field("data_file_id", &self.data_file_id)
            .field("offset", &format!("{:08X}", self.offset))
            .finish()
    }
}

///////////////////////////////////////////////

struct SqPackIndexHeader {
    pub index_data_offset: u32,
    pub index_data_size: u32,
}

impl SqPackIndexHeader {
    pub fn from_reader<R: ReadBytesExt>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let _size = reader.read_u32::<LittleEndian>()?;
        let _ptype = reader.read_u32::<LittleEndian>()?;
        let index_data_offset = reader.read_u32::<LittleEndian>()?;
        let index_data_size = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            index_data_offset,
            index_data_size,
        })
    }
}

///////////////////////////////////////////////

struct SqPackHeader {
    pub size: u32,
}

pub enum PlatformId {
    Win32,
    PS3,
    PS4,
}

impl SqPackHeader {
    pub fn from_reader<R: ReadBytesExt>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        let _magic = reader.read_u64::<LittleEndian>()?;
        let platform_id = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let _version = reader.read_u32::<LittleEndian>()?;
        let _ptype = reader.read_u32::<LittleEndian>()?;

        let _platform_id = match platform_id {
            0 => PlatformId::Win32,
            1 => PlatformId::PS3,
            2 => PlatformId::PS4,
            _ => panic!("Invalid PlatformID read from SqPack file"),
        };

        Ok(Self { size })
    }
}

///////////////////////////////////////////////

#[derive(Debug)]
struct Resource {
    pub directory: HashedString,
    pub file: HashedString,
    pub full_hash: HashedString,
}

impl Resource {
    pub fn new(path: impl AsRef<str>) -> Option<Self> {
        let path = path.as_ref();
        let (dir_path, file_path) = path.rsplit_once("/")?;
        Some(Self {
            directory: HashedString::new(dir_path),
            file: HashedString::new(file_path),
            full_hash: HashedString::new(path),
        })
    }
}

///////////////////////////////////////////////

struct HashedString {
    pub hash: u32,
    pub value: String,
}

impl HashedString {
    fn new(path: impl AsRef<str>) -> Self {
        let path = path.as_ref();
        Self {
            hash: crc32(path),
            value: path.to_string(),
        }
    }
}

impl Debug for HashedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entry(&self.value)
            .entry(&format!("{:08X}", self.hash))
            .finish()
    }
}

///////////////////////////////////////////////

type Crc32 = crc::Crc<u32>;
static CRC: Crc32 = Crc32::new(&crc::CRC_32_JAMCRC);
fn crc32(string: &str) -> u32 {
    CRC.checksum(&lower_case_string_bytes(string))
}

fn lower_case_string_bytes(string: &str) -> Box<[u8]> {
    string
        .to_lowercase()
        .bytes()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}
