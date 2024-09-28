use std::fmt::Display;

pub static REPOSITORIES: &[&str] = &[
    "ffxiv", "ex1", "ex2", "ex3", "ex4", "ex5", "ex6", "ex7", "ex8", "ex9",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileKey {
    pub category: Category,
    pub repository: Repository,
}

impl FileKey {
    pub fn new(path: impl AsRef<str>) -> Self {
        let path = path.as_ref();
        let (category, rest) = path.split_once("/").unwrap();
        let category = Category::from(category);

        let repository = match rest.split_once("/") {
            None => "ffxiv", // only a file_name e.g. exd/item.exh
            Some((repository, _)) => repository,
        };
        let repository = REPOSITORIES
            .iter()
            .find(|repo| **repo == repository)
            .cloned()
            .unwrap_or("ffxiv");
        let repository = Repository::from(repository);

        Self {
            category,
            repository,
        }
    }
}

impl Display for FileKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02x}", Into::<usize>::into(self.category))?;
        write!(f, "{:02x}", self.repository.0)
    }
}

///////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Common,
    BgCommon,
    Bg,
    Cutscene,
    Character,
    Shader,
    Ui,
    Sound,
    Vfx,
    UiScript,
    ExcelData,
    GameScript,
    Music,
    SqPackTest,
    Debug,
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Category::Common => "common",
                Category::BgCommon => "bgcommon",
                Category::Bg => "bg",
                Category::Cutscene => "cut",
                Category::Character => "chara",
                Category::Shader => "shader",
                Category::Ui => "ui",
                Category::Sound => "sound",
                Category::Vfx => "vfx",
                Category::UiScript => "ui_script",
                Category::ExcelData => "exd",
                Category::GameScript => "game_script",
                Category::Music => "music",
                Category::SqPackTest => "sqpack_test",
                Category::Debug => "debug",
            }
        )
    }
}

impl From<&str> for Category {
    fn from(value: &str) -> Self {
        match value {
            "common" => Category::Common,
            "bgcommon" => Category::BgCommon,
            "bg" => Category::Bg,
            "cut" => Category::Cutscene,
            "chara" => Category::Character,
            "shader" => Category::Shader,
            "ui" => Category::Ui,
            "sound" => Category::Sound,
            "vfx" => Category::Vfx,
            "ui_script" => Category::UiScript,
            "exd" => Category::ExcelData,
            "game_script" => Category::GameScript,
            "music" => Category::Music,
            "sqpack_test" => Category::SqPackTest,
            "debug" => Category::Debug,
            _ => panic!("Unexpected value for Category: {}", value),
        }
    }
}

impl From<usize> for Category {
    fn from(value: usize) -> Self {
        match value {
            0 => Category::Common,
            1 => Category::BgCommon,
            2 => Category::Bg,
            3 => Category::Cutscene,
            4 => Category::Character,
            5 => Category::Shader,
            6 => Category::Ui,
            7 => Category::Sound,
            8 => Category::Vfx,
            9 => Category::UiScript,
            10 => Category::ExcelData,
            11 => Category::GameScript,
            12 => Category::Music,
            18 => Category::SqPackTest,
            19 => Category::Debug,
            _ => panic!("Unknown Category: {}", value),
        }
    }
}

impl From<u32> for Category {
    fn from(value: u32) -> Self {
        Self::from(value as usize)
    }
}

impl From<Category> for usize {
    fn from(value: Category) -> Self {
        match value {
            Category::Common => 0,
            Category::BgCommon => 1,
            Category::Bg => 2,
            Category::Cutscene => 3,
            Category::Character => 4,
            Category::Shader => 5,
            Category::Ui => 6,
            Category::Sound => 7,
            Category::Vfx => 8,
            Category::UiScript => 9,
            Category::ExcelData => 10,
            Category::GameScript => 11,
            Category::Music => 12,
            Category::SqPackTest => 18,
            Category::Debug => 19,
        }
    }
}

///////////////////////////////////////////////

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Repository(usize);

impl From<usize> for Repository {
    fn from(value: usize) -> Self {
        Repository(value)
    }
}

impl From<&str> for Repository {
    fn from(value: &str) -> Self {
        Repository(match value {
            "ffxiv" => 0,
            "ex1" => 1,
            "ex2" => 2,
            "ex3" => 3,
            "ex4" => 4,
            "ex5" => 5,
            "ex6" => 6,
            "ex7" => 7,
            "ex8" => 8,
            "ex9" => 9,
            _ => panic!("Unexpected Repository value: {}", value),
        })
    }
}

impl From<String> for Repository {
    fn from(value: String) -> Self {
        Repository::from(value.as_ref())
    }
}

impl Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                0 => "ffxiv".to_string(),
                ex => format!("ex{}", ex),
            }
        )
    }
}
