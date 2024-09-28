use std::{
    collections::HashMap,
    error::Error,
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    excel::{ExcelDataFile, ExcelDataRow, ExcelDataType, ExcelHeaderFile, ExcelLanguage},
    ffxiv_file::FfxivFile,
    file_key::FileKey,
    sqpack::SqPackIndexFile,
};

type Reader = BufReader<std::fs::File>;

pub struct FfxivLibrary {
    game_path: PathBuf,
    index_files: HashMap<FileKey, SqPackIndexFile>,
    dat_files: HashMap<(FileKey, u32), Reader>,
}

impl FfxivLibrary {
    pub fn new(game_path: impl AsRef<Path>) -> Self {
        Self {
            game_path: game_path.as_ref().to_path_buf(),
            index_files: HashMap::new(),
            dat_files: HashMap::new(),
        }
    }

    pub fn get_file(&mut self, path: impl AsRef<str>) -> Result<FfxivFile, Box<dyn Error>> {
        let path = path.as_ref();
        let file_key = FileKey::new(path);
        let file_path = format!("{}00.win32", file_key);

        if let std::collections::hash_map::Entry::Vacant(e) = self.index_files.entry(file_key) {
            let file_path = format!(
                "{}/{}/{}.index",
                self.game_path.to_str().unwrap(),
                file_key.repository,
                file_path
            );
            e.insert(SqPackIndexFile::from_file(file_path)?);
        }
        let index_file = self.index_files.get(&file_key).unwrap();

        let entry = index_file.entry_from_path(path).ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found: {}", path),
        ))?;

        if let std::collections::hash_map::Entry::Vacant(e) =
            self.dat_files.entry((file_key, entry.data_file_id))
        {
            let file_path = format!(
                "{}/{}/{}.dat{}",
                self.game_path.to_str().unwrap(),
                file_key.repository,
                file_path,
                entry.data_file_id,
            );
            e.insert(BufReader::new(std::fs::File::open(file_path)?));
        }
        let reader = self
            .dat_files
            .get_mut(&(file_key, entry.data_file_id))
            .unwrap();

        let file = FfxivFile::from_reader(reader, path, entry)?;
        Ok(file)
    }

    pub fn get_table_data(
        &mut self,
        path: impl AsRef<str>,
    ) -> Result<Vec<ExcelDataRow>, Box<dyn Error>> {
        let path = path.as_ref();
        let header_file_path = format!("{}.exh", path);
        let excel_file = ExcelHeaderFile::from_file(self.get_file(&header_file_path)?)?;

        let mut vec = Vec::new();
        for excel_page in &excel_file.pages {
            let language_ending = excel_file
                .languages
                .contains(&ExcelLanguage::English)
                .then_some(ExcelLanguage::English)
                .map(|lang| lang.as_country_code())
                .unwrap_or_else(|| ExcelLanguage::None.as_country_code());

            let path = format!(
                "{}_{}{}.exd",
                path, excel_page.start_row_id, language_ending
            );
            let file = match self.get_file(&path) {
                Ok(v) => v,
                Err(e) => {
                    println!("Error for {}: {:?}", path, e);
                    continue;
                }
            };

            let excel_data_file = ExcelDataFile::from_file(file, &excel_file)?;
            vec.append(&mut excel_data_file.into_inner());
        }

        Ok(vec)
    }

    pub fn write_to_csv(&mut self, path: impl AsRef<str>) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();
        let data: Vec<ExcelDataRow> = self.get_table_data(path)?;

        let mut writer = BufWriter::new(std::fs::File::create(format!("out/{}.csv", path))?);
        for row in data {
            let entries = [format!("{}", row.row_info().row_id)]
                .into_iter()
                .chain(row.iter().map(|entry| match entry {
                    ExcelDataType::String(v) => format!("\"{}\"", v),
                    ExcelDataType::I64(v) => format!("{}", v),
                    ExcelDataType::U64(v) => format!("{}", v),
                    ExcelDataType::F32(v) => format!("{}", v),
                }))
                .collect::<Vec<_>>()
                .join(",");

            writeln!(writer, "{}", entries)?;
        }

        Ok(())
    }
}
