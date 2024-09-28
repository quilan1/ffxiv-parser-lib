use std::{
    error::Error,
    io::{Read, Seek, Write},
    ops::Deref,
};

use byteorder::{LittleEndian, ReadBytesExt};
use flate2::bufread::DeflateDecoder;

use crate::sqpack::SqPackIndexTableEntry;

//////////////////////////////////////////

pub struct FfxivFile(String, Box<[u8]>);

impl FfxivFile {
    pub fn from_reader(
        reader: &mut (impl ReadBytesExt + Seek),
        path: impl AsRef<str>,
        entry: &SqPackIndexTableEntry,
    ) -> Result<Self, Box<dyn Error>> {
        reader.seek(std::io::SeekFrom::Start(entry.offset as u64))?;
        let path = path.as_ref();

        let header = CommonHeader::from_reader(reader)?;
        let FileType::Standard = header.file_type else {
            unimplemented!()
        };

        let block_info = (0..header.block_count)
            .flat_map(|_| BlockInfo::from_reader(reader))
            .collect::<Vec<_>>();

        let blocks_offset = (entry.offset + header.size) as u64;

        let mut file_contents = Vec::new();
        for info in block_info {
            reader.seek(std::io::SeekFrom::Start(blocks_offset + info.offset as u64))?;
            let block_header = BlockHeader::from_reader(reader)?;
            let block_data = BlockData::from_reader(reader, &block_header)?;

            let mut data = Vec::new();
            let mut decoder = DeflateDecoder::new(&block_data[..]);
            decoder.read_to_end(&mut data)?;
            file_contents.append(&mut data);
        }

        Ok(Self(path.to_string(), file_contents.into_boxed_slice()))
    }

    #[allow(dead_code)]
    pub fn file_name(&self) -> &str {
        &self.0
    }

    #[allow(dead_code)]
    pub fn write(&self) -> Result<(), Box<dyn Error>> {
        let path = format!("out/{}", self.0);
        println!("Writing to {}", path);
        let mut out_file = std::io::BufWriter::new(std::fs::File::create(path)?);
        out_file.write_all(&self.1)?;
        Ok(())
    }
}

impl Deref for FfxivFile {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

//////////////////////////////////////////

struct BlockInfo {
    pub offset: u32,
}

impl BlockInfo {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let offset = reader.read_u32::<LittleEndian>()?;
        let _ = reader.read_u16::<LittleEndian>()?;
        let _size = reader.read_u16::<LittleEndian>()?;
        Ok(Self { offset })
    }
}

//////////////////////////////////////////

#[allow(dead_code)]
struct BlockHeader {
    pub compressed_size: u32,
    pub uncompressed_size: u32,
}

impl BlockHeader {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let _size = reader.read_u32::<LittleEndian>()?;
        let _ = reader.read_u32::<LittleEndian>()?;
        let compressed_size = reader.read_u32::<LittleEndian>()?;
        let uncompressed_size = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            compressed_size,
            uncompressed_size,
        })
    }
}

//////////////////////////////////////////

struct BlockData(Box<[u8]>);

impl BlockData {
    pub fn from_reader(
        reader: &mut impl ReadBytesExt,
        block_header: &BlockHeader,
    ) -> Result<Self, Box<dyn Error>> {
        let mut data = vec![0; block_header.compressed_size as usize];
        reader.read_exact(data.as_mut_slice())?;
        Ok(Self(data.into_boxed_slice()))
    }
}

impl Deref for BlockData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//////////////////////////////////////////

#[allow(dead_code)]
#[derive(Debug)]
struct CommonHeader {
    pub size: u32,
    pub file_type: FileType,
    pub file_size: u32,
    pub block_count: u32,
}

impl CommonHeader {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let size = reader.read_u32::<LittleEndian>()?;
        let file_type = reader.read_u32::<LittleEndian>()?;
        let file_size = reader.read_u32::<LittleEndian>()?;
        let _num_blocks = reader.read_u32::<LittleEndian>()?;
        let _block_buffer_size = reader.read_u32::<LittleEndian>()?;
        let block_count = reader.read_u32::<LittleEndian>()?;

        let file_type = FileType::from(file_type);

        Ok(Self {
            size,
            file_type,
            file_size,
            block_count,
        })
    }
}

#[derive(Debug)]
enum FileType {
    Empty = 1,
    Standard,
    Model,
    Texture,
}

impl From<u32> for FileType {
    fn from(value: u32) -> Self {
        match value {
            1 => FileType::Empty,
            2 => FileType::Standard,
            3 => FileType::Model,
            4 => FileType::Texture,
            _ => panic!("Unexpected file type: {}", value),
        }
    }
}

//////////////////////////////////////////
