use std::{path::Path, time::{SystemTime, UNIX_EPOCH}};

use crate::common::config;

pub struct PathUtil {
    prefix: String,
}

impl PathUtil {
    pub fn new(prefix: &String) -> Self {
        Self { prefix: prefix.clone() }
    }
}

impl PathUtil {
    pub fn full_path(&self, path: &String) -> String {
        let mut path = path.clone();
        while path.contains("../") {
            path = path.replace("../", "");
        }
        path = path.trim_end_matches(".").to_string();
        path = path.trim_end_matches("/").to_string();
        path = path.trim_start_matches(".").to_string();
        path = path.trim_start_matches("/").to_string();
        String::from(Path::new(&self.prefix).join(path.clone()).to_str().unwrap())
    }

    pub fn mask_path(&self, path: &String) -> String {
        (*path).replace(self.prefix.as_str(), "/")
    }
}

pub fn config_dir() -> String {
    match dirs::config_dir() {
        Some(path) => {
            format!("{}/{}", path.display(), config::FILE_TUNNEL_APP_NAME)
        },
        None => format!("{}/{}/{}", config::WORK_DIR, config::CONFIG_DIR, config::FILE_TUNNEL_APP_NAME)
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub fn format_size(bytes: u64) -> String {
    let size_units = vec!["GB","MB","KB","B"];
    let size_units_len = size_units.len() - 1;
    let mut size = "".to_string();
    for (idx, size_unit) in size_units.into_iter().enumerate() {
        if bytes > (1<<(10 * (size_units_len - idx))) {
            size = format!("{}{}", bytes >> (10 * (size_units_len - idx)), size_unit);
            break ;
        }
    }

    size
}
