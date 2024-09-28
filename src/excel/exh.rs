use std::{error::Error, fmt::Display};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use crate::ffxiv_file::FfxivFile;

//////////////////////////////////////////

#[derive(Debug)]
pub struct ExcelHeaderFile {
    pub header: ExcelHeader,
    pub columns: Vec<ExcelColumn>,
    pub pages: Vec<ExcelPageInfo>,
    pub languages: Vec<ExcelLanguage>,
}

impl ExcelHeaderFile {
    pub fn from_file(file: FfxivFile) -> Result<Self, Box<dyn Error>> {
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(&file[..]));
        Self::from_reader(&mut reader)
    }

    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let header = ExcelHeader::from_reader(reader)?;
        let columns = (0..header.column_count)
            .flat_map(|_| ExcelColumn::from_reader(reader))
            .collect::<Vec<_>>();
        let pages = (0..header.page_count)
            .flat_map(|_| ExcelPageInfo::from_reader(reader))
            .collect::<Vec<_>>();
        let languages = (0..header.language_count)
            .flat_map(|_| ExcelLanguage::from_reader(reader))
            .collect::<Vec<_>>();

        Ok(Self {
            header,
            columns,
            pages,
            languages,
        })
    }
}

//////////////////////////////////////////

#[derive(Debug)]
pub struct ExcelColumn {
    pub data_type: ExcelColumnDataType,
    pub offset: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum ExcelColumnDataType {
    String = 0x0,
    Bool = 0x1,
    Int8 = 0x2,
    UInt8 = 0x3,
    Int16 = 0x4,
    UInt16 = 0x5,
    Int32 = 0x6,
    UInt32 = 0x7,
    Float32 = 0x9,
    Int64 = 0xA,
    UInt64 = 0xB,

    // 0 is read like data & 1, 1 is like data & 2, 2 = data & 4, etc...
    PackedBool0 = 0x19,
    PackedBool1 = 0x1A,
    PackedBool2 = 0x1B,
    PackedBool3 = 0x1C,
    PackedBool4 = 0x1D,
    PackedBool5 = 0x1E,
    PackedBool6 = 0x1F,
    PackedBool7 = 0x20,
}

impl ExcelColumn {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let data_type = reader.read_u16::<BigEndian>()?;
        let offset = reader.read_u16::<BigEndian>()?;

        let data_type = match data_type {
            0x0 => ExcelColumnDataType::String,
            0x1 => ExcelColumnDataType::Bool,
            0x2 => ExcelColumnDataType::Int8,
            0x3 => ExcelColumnDataType::UInt8,
            0x4 => ExcelColumnDataType::Int16,
            0x5 => ExcelColumnDataType::UInt16,
            0x6 => ExcelColumnDataType::Int32,
            0x7 => ExcelColumnDataType::UInt32,
            0x9 => ExcelColumnDataType::Float32,
            0xA => ExcelColumnDataType::Int64,
            0xB => ExcelColumnDataType::UInt64,
            0x19 => ExcelColumnDataType::PackedBool0,
            0x1A => ExcelColumnDataType::PackedBool1,
            0x1B => ExcelColumnDataType::PackedBool2,
            0x1C => ExcelColumnDataType::PackedBool3,
            0x1D => ExcelColumnDataType::PackedBool4,
            0x1E => ExcelColumnDataType::PackedBool5,
            0x1F => ExcelColumnDataType::PackedBool6,
            0x20 => ExcelColumnDataType::PackedBool7,
            _ => panic!("Unexpected data type: {}", data_type),
        };

        Ok(Self { data_type, offset })
    }
}

//////////////////////////////////////////

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExcelHeader {
    pub data_offset: u16,
    pub column_count: u16,
    pub page_count: u16,
    pub language_count: u16,
    pub variant: ExcelVariant,
    pub row_count: u32,
}

#[derive(Debug)]
pub enum ExcelVariant {
    Default = 1,
    SubRows,
}

#[derive(Debug)]
pub enum ExcelHeaderError {
    InvalidMagic,
}

impl ExcelHeader {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x45584846 {
            return Err(Box::new(ExcelHeaderError::InvalidMagic));
        }
        let _ = reader.read_u16::<BigEndian>()?;
        let data_offset = reader.read_u16::<BigEndian>()?;
        let column_count = reader.read_u16::<BigEndian>()?;
        let page_count = reader.read_u16::<BigEndian>()?;
        let language_count = reader.read_u16::<BigEndian>()?;
        let _ = reader.read_u16::<BigEndian>()?;
        let _ = reader.read_u8()?;
        let variant = reader.read_u8()?;
        let _ = reader.read_u16::<BigEndian>()?;
        let row_count = reader.read_u32::<BigEndian>()?;
        let _ = [
            reader.read_u32::<BigEndian>()?,
            reader.read_u32::<BigEndian>()?,
        ];

        let variant = match variant {
            1 => ExcelVariant::Default,
            2 => ExcelVariant::SubRows,
            _ => panic!("Invalid variant type: {}", variant),
        };

        Ok(Self {
            data_offset,
            column_count,
            page_count,
            language_count,
            variant,
            row_count,
        })
    }
}

impl Error for ExcelHeaderError {}

impl Display for ExcelHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                ExcelHeaderError::InvalidMagic => "InvalidMagic",
            }
        )
    }
}

//////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub enum ExcelLanguage {
    None,
    Japanese,
    English,
    German,
    French,
    ChineseSimplified,
    ChineseTraditional,
    Korean,
}

impl ExcelLanguage {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let language = reader.read_u16::<LittleEndian>()?;

        Ok(match language {
            0 => ExcelLanguage::None,
            1 => ExcelLanguage::Japanese,
            2 => ExcelLanguage::English,
            3 => ExcelLanguage::German,
            4 => ExcelLanguage::French,
            5 => ExcelLanguage::ChineseSimplified,
            6 => ExcelLanguage::ChineseTraditional,
            7 => ExcelLanguage::Korean,
            _ => panic!("Unexpected language value: {}", language),
        })
    }

    pub fn as_country_code(&self) -> &'static str {
        match *self {
            ExcelLanguage::None => "",
            ExcelLanguage::Japanese => "_jp",
            ExcelLanguage::English => "_en",
            ExcelLanguage::German => "_de",
            ExcelLanguage::French => "_fr",
            ExcelLanguage::ChineseSimplified => "_ch",
            ExcelLanguage::ChineseTraditional => "_ch",
            ExcelLanguage::Korean => "_kr",
        }
    }
}

//////////////////////////////////////////

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExcelPageInfo {
    pub start_row_id: u32,
    pub row_count: u32,
}

impl ExcelPageInfo {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let start_row_id = reader.read_u32::<BigEndian>()?;
        let row_count = reader.read_u32::<BigEndian>()?;

        Ok(Self {
            start_row_id,
            row_count,
        })
    }
}
