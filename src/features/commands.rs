use std::{
    fmt::Display, fs::{self, DirEntry, Metadata}, path::{Path, PathBuf}, str::FromStr, time::UNIX_EPOCH
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct WebSocketCommand {
    pub version: u16,
    pub command: Command
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct FtPath {
    root: String,
    relative_path: String,
}

impl Display for FtPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",  self.full_path())
    }
}

impl FtPath {
    pub fn new_relative(root: String, relative_path: String) -> Self {
        let relative_path = relative_path.trim_start_matches("/").to_string();
        Self { root, relative_path }
    }

    pub fn new_absolute(root: String, absolute_path: String) -> Self {
        let relative_path = absolute_path.replace(&root, "");
        Self::new_relative(root, relative_path)
    }

    pub fn create_relative(&self, relative_path: String) -> Self {
        Self::new_relative(self.root.clone(), relative_path)
    }

    pub fn create_absolute(&self, absolute_path: String) -> Self {
        Self::new_absolute(self.root.clone(), absolute_path)
    }
}

impl FtPath {
    pub fn full_path(&self) -> String {
        let path = PathBuf::from(self.root.clone());
        // note: 如果 self.relative_path 是 绝对路径，join 会替换，不会拼接
        //  故而，此处需替换开头的 路径分割符
        let relative_path = self.relative_path.trim_start_matches("/").to_string();
        path.join(relative_path).to_str().unwrap().to_string()
    }

    pub fn path(&self) -> String {
        self.relative_path.clone()
    }

    pub fn reset_root(&mut self, root: &str) {
        self.root = root.to_string();
    }

    pub fn replace_root_path(&mut self, new_root: &str) {
        self.relative_path = self.relative_path.trim_start_matches(&new_root).to_string();
    }

    pub fn root_path(&self) -> &String {
        &self.root
    }

    pub fn exists(&self) -> bool {
        Path::new(&self.full_path()).exists()
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum Command {
    ReadConfig {},
    ReadDirItem {
        dir_path: FtPath,
        take_size: usize,
        skip_size: usize,
    },
    ReadFileInfo {
        file_path: FtPath,
    },
    ReadPathInfo {
        path: FtPath,
        take_size: usize,
        skip_size: usize,
    },
    DownloadFile {
        file_path: FtPath,
        block_idx: usize,
        block_size: usize,
    },
    ModifiedFile {
        path: FtPath,
        m_type: ModfiedType,
    }
}


#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct CommandMessage {
    pub version: u16,
    pub status: u16,
    pub data: CommandData,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub struct DirItem {
    pub path: FtPath,
    pub info: DirItemInfo
}

impl DirItem {
    pub fn path(&self) -> &FtPath {
        &self.path
    }
}

impl From <DirEntry> for DirItem {
    fn from(value: DirEntry) -> Self {
        let path = value.path().to_str().unwrap().to_string();
        Self {
            info: DirItemInfo::new(&path),
            path: FtPath::new_absolute("".to_string(), path),
        }
    }
}

impl From<String> for DirItem {
    fn from(value: String) -> Self {
        Self {
            info: DirItemInfo::new(&value),
            path: FtPath::new_absolute("".to_string(), value),
        }
    }
}

impl From<&str> for DirItem {
    fn from(value: &str) -> Self {
        Self {
            path: FtPath::new_absolute("".to_string(), value.to_string()),
            info: DirItemInfo::new(&value.to_string())
        }
    }
}

impl From<FtPath> for DirItem {
    fn from(value: FtPath) -> Self {
        Self {
            info: DirItemInfo::new(&value.full_path()),
            path: value,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug, Clone)]
pub enum DirItemInfo {
    Dir {
        modified_at: u64,
        created_at: u64,
        item_count: u64,
    },
    File {
        modified_at: u64,
        created_at: u64,
        file_size: u64,
        chksum: String,
    }
}

impl DirItemInfo {
    pub fn new(path: &String) -> Self {
        let _path = Path::new(path);
        let meta = _path.metadata().expect(format!("path {}", path).as_str());
        let modified_at = meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as u64;
        let created_at = meta.created().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as u64;
        if _path.is_dir() {
            return Self::Dir {
                modified_at: modified_at,
                created_at: created_at,
                item_count: Self::item_size(path, &meta),
            };
        }
        if _path.is_file() {
            use sha256::try_digest;
            return Self::File {
                modified_at: modified_at,
                created_at: created_at,
                file_size: Self::item_size(path, &meta),
                chksum: try_digest(_path).unwrap(),
            };
        }
        panic!("{} unsupported type", path);
    }

    pub fn item_size(path: &String, meta: &Metadata) -> u64 {
        if meta.is_dir() {
            let mut _size = 0;
            let _: Vec<_> = fs::read_dir(path).unwrap().map(|f| {
                if let Ok(_f) = f {
                    _size += 1;
                }
            }).collect();

            return _size;
        } else if meta.is_file() {
            return meta.len() as u64;
        } else {
            return 0;
        }
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum CommandData {
    ReadConfig {
        path: String,
    },
    ReadDirItem {
        items: Vec<DirItem>,
        total: usize,
        taked_size: usize,
    },
    ReadFileInfo {
        item: DirItem,
    },
    DownloadFile {
        data: Vec<u8>,
        data_size: usize,
    },
    ModifiedFile {
        path: String,
        m_type: ModfiedType,
    },
    Error {
        message: String,
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub enum ModfiedType {
    Meta,
    Content,
}

#[cfg(test)]
mod test_commands {
    use crate::features::commands::{FtPath, ModfiedType};

    use super::{Command, WebSocketCommand};

    #[test]
    fn se_commands() {
        let cmd = WebSocketCommand {
            version: 1,
            command: Command::ModifiedFile { path: FtPath::new_absolute("".to_string(), "".to_string()), m_type: ModfiedType::Content }
        };

        println!("json: {}", serde_json::to_string(&cmd).unwrap());
    }

    #[test]
    fn de_commands() {
        let s = r#"{"version":1,"command":{"ModifiedFile":{"path":"","m_type":"Content"}}}"#;
        
        let cmd: WebSocketCommand = serde_json::from_str(s).unwrap();
        println!("{cmd:#?}");

    }

}