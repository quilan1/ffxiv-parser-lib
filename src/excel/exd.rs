use std::{error::Error, fmt::Display, io::Seek, ops::Deref};

use byteorder::{BigEndian, ReadBytesExt};

use crate::ffxiv_file::FfxivFile;

use super::{ExcelColumn, ExcelColumnDataType, ExcelHeaderFile};

//////////////////////////////////////////

pub struct ExcelDataFile(Vec<ExcelDataRow>);

impl ExcelDataFile {
    pub fn from_file(
        file: FfxivFile,
        excel_file: &ExcelHeaderFile,
    ) -> Result<Self, Box<dyn Error>> {
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(&file[..]));
        Self::from_reader(&mut reader, excel_file)
    }

    pub fn from_reader<R: ReadBytesExt + Seek>(
        reader: &mut R,
        excel_file: &ExcelHeaderFile,
    ) -> Result<Self, Box<dyn Error>> {
        let data_header = ExcelDataHeader::from_reader(reader)?;
        let row_infos = (0..data_header.num_rows)
            .flat_map(|_| ExcelRowInfo::from_reader(reader))
            .collect::<Vec<_>>();

        let data_offset = excel_file.header.data_offset as u64;
        let column_data = &excel_file.columns;

        let mut data = Vec::new();
        for row_info in row_infos {
            data.push(ExcelDataRow::from_reader(
                reader,
                row_info,
                column_data,
                data_offset,
            )?);
        }

        Ok(Self(data))
    }

    pub fn into_inner(self) -> Vec<ExcelDataRow> {
        self.0
    }
}

//////////////////////////////////////////

#[derive(Debug)]
pub struct ExcelDataRow(Vec<ExcelDataType>, ExcelRowInfo);

#[derive(Debug)]
pub enum ExcelDataType {
    String(String),
    U64(u64),
    I64(i64),
    F32(f32),
}

impl ExcelDataRow {
    pub fn from_reader(
        reader: &mut (impl ReadBytesExt + Seek),
        row_info: ExcelRowInfo,
        column_data: &[ExcelColumn],
        data_offset: u64,
    ) -> Result<Self, Box<dyn Error>> {
        let row_data_start = row_info.offset as u64 + 6;
        let row_data_end = row_data_start + data_offset;

        let data = column_data
            .iter()
            .flat_map(|excel_column| {
                read_cell_data(reader, row_data_start, row_data_end, excel_column)
            })
            .collect::<Vec<_>>();

        Ok(Self(data, row_info))
    }

    pub fn row_info(&self) -> &ExcelRowInfo {
        &self.1
    }
}

fn read_cell_data(
    reader: &mut (impl ReadBytesExt + Seek),
    row_data_start: u64,
    row_data_end: u64,
    excel_column: &ExcelColumn,
) -> Result<ExcelDataType, Box<dyn Error>> {
    let start_offset = row_data_start + excel_column.offset as u64;
    reader.seek(std::io::SeekFrom::Start(start_offset))?;
    Ok(match excel_column.data_type {
        ExcelColumnDataType::String => {
            let data_offset = reader.read_u32::<BigEndian>()?;
            let string_offset = row_data_end + data_offset as u64;
            reader.seek(std::io::SeekFrom::Start(string_offset))?;

            let mut buf = Vec::new();
            loop {
                let value = reader.read_u8()?;
                if value == 0 {
                    break;
                }
                buf.push(value);
            }
            ExcelDataType::String(String::from_utf8(buf)?)
        }
        ExcelColumnDataType::Int8 => ExcelDataType::I64(reader.read_i8()? as i64),
        ExcelColumnDataType::Int16 => ExcelDataType::I64(reader.read_i16::<BigEndian>()? as i64),
        ExcelColumnDataType::Int32 => ExcelDataType::I64(reader.read_i32::<BigEndian>()? as i64),
        ExcelColumnDataType::Int64 => ExcelDataType::I64(reader.read_i64::<BigEndian>()?),

        ExcelColumnDataType::Bool | ExcelColumnDataType::UInt8 => {
            ExcelDataType::U64(reader.read_u8()? as u64)
        }
        ExcelColumnDataType::UInt16 => ExcelDataType::U64(reader.read_u16::<BigEndian>()? as u64),
        ExcelColumnDataType::UInt32 => ExcelDataType::U64(reader.read_u32::<BigEndian>()? as u64),
        ExcelColumnDataType::UInt64 => ExcelDataType::U64(reader.read_u64::<BigEndian>()?),

        ExcelColumnDataType::Float32 => ExcelDataType::F32(reader.read_f32::<BigEndian>()?),

        ExcelColumnDataType::PackedBool0
        | ExcelColumnDataType::PackedBool1
        | ExcelColumnDataType::PackedBool2
        | ExcelColumnDataType::PackedBool3
        | ExcelColumnDataType::PackedBool4
        | ExcelColumnDataType::PackedBool5
        | ExcelColumnDataType::PackedBool6
        | ExcelColumnDataType::PackedBool7 => {
            let bit = excel_column.data_type as u8 - ExcelColumnDataType::PackedBool0 as u8;
            let data = reader.read_u8()?;
            ExcelDataType::U64(((data & (1 << bit)) > 0) as u64)
        }
    })
}

impl Deref for ExcelDataRow {
    type Target = [ExcelDataType];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//////////////////////////////////////////

#[derive(Debug)]
pub struct ExcelRowInfo {
    pub row_id: u32,
    pub offset: u32,
}

impl ExcelRowInfo {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let row_id = reader.read_u32::<BigEndian>()?;
        let offset = reader.read_u32::<BigEndian>()?;
        Ok(Self { row_id, offset })
    }
}

//////////////////////////////////////////

struct ExcelDataHeader {
    num_rows: u32,
}

#[derive(Debug)]
pub enum ExcelHeaderError {
    InvalidMagic,
}

impl ExcelDataHeader {
    pub fn from_reader(reader: &mut impl ReadBytesExt) -> Result<Self, Box<dyn Error>> {
        let magic = reader.read_u32::<BigEndian>()?;
        if magic != 0x45584446 {
            return Err(Box::new(ExcelHeaderError::InvalidMagic));
        }
        let _version = reader.read_u16::<BigEndian>()?;
        let _u1 = reader.read_u16::<BigEndian>()?;
        let row_info_size = reader.read_u32::<BigEndian>()?;
        let _u2 = (0..10)
            .flat_map(|_| reader.read_u16::<BigEndian>())
            .collect::<Vec<_>>();

        let num_rows = row_info_size / 8;

        Ok(Self { num_rows })
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
