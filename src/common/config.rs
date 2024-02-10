use std::{collections::HashMap, fs, sync::{Arc, Mutex}};

use sqlite::{State, Connection};

pub const WORK_DIR: &str = "./work_dir";

pub const CONFIG_DIR: &str = ".config";

pub const FILE_TUNNEL_APP_NAME: &str = "file-tunnel";

pub const FILE_TUNNEL_ENDPOINT_CLIENT: &str = "client";
pub const FILE_TUNNEL_ENDPOINT_SERVER: &str = "server";
pub const FILE_TUNNEL_ENDPOINT_TUNNEL: &str = "tunnel";

pub const FILE_TUNNEL_CFG_CLIENT: &str = "client.db";
pub const FILE_TUNNEL_CFG_SERVER: &str = "server.db";
pub const FILE_TUNNEL_CFG_TUNNEL: &str = "tunnel.db";

pub const CFG_PATH: &str = "path";
pub const CFG_TUNNEL_HOST: &str = "tunnel_host";
pub const CFG_SHARE_KEY: &str = "share_key";
pub const CFG_PASSWORD: &str = "password";
pub const CFG_CLIENT_KEY: &str = "client_id";
const SERVER_ALLOW_NAMES: [&str; 4] = [
    CFG_PATH, CFG_TUNNEL_HOST, CFG_SHARE_KEY, CFG_PASSWORD,
];

const CLIENT_ALLOW_NAMES: [&str; 5] = [
    CFG_PATH, CFG_TUNNEL_HOST, CFG_SHARE_KEY, CFG_PASSWORD,
    CFG_CLIENT_KEY,
];

#[derive(Clone)]
pub struct Config {
    work_dir: String,
    config_path: String,
    allowed_names: Vec<String>,
    config_dict: HashMap<String, String>,
    dirty: Vec<(String, String, i64)>,
    conn: Arc<Mutex<Connection>>
}

impl Config {
    pub fn new(
        work_dir: Option<String>,
        config_file: Option<String>,
        app_name: Option<String>
    ) -> Self {
        let work_dir = match work_dir {
            Some(work_dir) => work_dir,
            None => WORK_DIR.to_string(),
        };
        
        let allowed_names: Vec<String> = match app_name {
            Some(app_name) => {
                match app_name.as_str() {
                    FILE_TUNNEL_ENDPOINT_CLIENT => CLIENT_ALLOW_NAMES
                        .iter()
                        .map(|an| an.to_string()).collect(),
                    FILE_TUNNEL_ENDPOINT_SERVER => SERVER_ALLOW_NAMES
                        .iter()
                        .map(|an| an.to_string()).collect(),
                    _ => todo!("todo {app_name}")
                }
            },
            None => vec![],
        };
        
        let config_file = if let Some(_config_file) = config_file {
            _config_file
        } else { ".ft.db".to_string() };
        let config_path = format!(
            "{}/{}",
            work_dir,
            config_file
        );
        if let Err(err) = fs::metadata(&work_dir) {
            eprintln!("warning: {} {}", work_dir, err);
            fs::create_dir_all(&work_dir).expect("app init failed, message: create work dir failed");
        }
        if let Err(err) = fs::metadata(&config_path) {
            eprintln!("warnning: config path error {}, error: {}", &config_path, err);
            fs::File::create(&config_path).expect("create config file failed");
        }
        let conn = sqlite::open(&config_path.clone()).expect("config connectiont failed");
        Self { 
            work_dir: work_dir.to_owned(),
            config_path: config_path.to_owned(),
            allowed_names: allowed_names.clone(), 
            config_dict: HashMap::new(),
            dirty: vec![],
            conn: Arc::new(Mutex::new(conn)),
        }
    }
}

impl Config {
    pub fn init(&mut self) {
        self.conn.lock().unwrap().execute(r#"
            create table if not exists config (
                name char(32) NOT NULL,
                value TEXT NOT NULL,
                auto_gen INT NOT NULL DEFAULT 0
            );
        "#).expect("init config table failed");
    }
    
    pub fn set(&mut self, key: String, value: String, auto_gen: Option<i64>) {
        let auto_gen: i64 = if let Some(auto_gen) = auto_gen {
            auto_gen
        } else { 0 };
        match self.config_dict.get_mut(&key) {
            Some(_v) => {
                if *_v != value {
                    *_v = value.clone();
                    self.dirty.push((key, value, auto_gen));
                }
            },
            None => { 
                self.config_dict.insert(key.clone(), value.clone());
                self.dirty.push((key, value, auto_gen));
            },
        }
        self.flush_dirty();
    }

    pub fn get_key(&mut self, key: String) -> Option<String> {
        match self.config_dict.get(&key) {
            Some(val) => Some(val.clone()),
            None => {
                let query = "select value from config where name = ?";
                if let Ok(conn) = self.conn.lock() {
                    let mut stat = conn.prepare(query).unwrap();
                    stat.bind((1, key.as_str())).unwrap();
                    while let Ok(State::Row) = stat.next() {
                        let val = stat.read::<String, _>("value").unwrap();
                        self.config_dict.insert(key, val.clone());
                        return Some(val);
                    }
                }
                None
            }
        }
    }

    pub fn get_keys(&mut self, keys: Option<Vec<String>>) -> Vec<(String, String)> {
        let keys: Vec<String> = if let Some(keys) = keys {
            keys.iter()
                .filter(|k|self.allowed_names.contains(k))
                .map(|k| k.clone())
                .collect()
        } else { 
            self.allowed_names.iter()
                .filter(|k| **k != CFG_PASSWORD.to_string())
                .map(|k| k.clone())
                .collect()
        };

        let mut out_mem_keys: Vec<String> = vec![];
        let mut key_value_s: Vec<(String, String)> = vec![];
        let _: Vec<_> = keys.iter().map(|k| {
            match self.config_dict.get(k) {
                Some(_v) => key_value_s.push((k.clone(), _v.clone())),
                None => out_mem_keys.push(k.clone()),
            }
        }).collect();

        if !out_mem_keys.is_empty() {
            let qmarks: Vec<&str> = vec!["?"; out_mem_keys.len()];
            let query = format!(
                "select name, value from config where name in ({})",
                qmarks.join(",")
            );
            if let Ok(conn) = self.conn.lock() {
                let mut stat = conn.prepare(query).unwrap();
                for (idx, name) in out_mem_keys.into_iter().enumerate() {
                    stat.bind((idx+1, name.as_str())).unwrap();
                }
                while let Ok(State:: Row) = stat.next() {
                    let key = stat.read::<String, _>("name").unwrap();
                    let value = stat.read::<String, _>("value").unwrap();
                    self.config_dict.insert(key.clone(), value.clone());
                    key_value_s.push((key, value));
                }
            }
        }

        key_value_s
    }

    pub fn get_keys_to_map(&mut self, keys: Option<Vec<String>>) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let _: Vec<_> = self.get_keys(keys).iter().map(|(key, value)| {
            map.insert(key.clone(), value.clone());
        }).collect();
        map
    }

    pub fn keys(&self) -> &Vec<String> {
        &self.allowed_names
    }

    fn flush_dirty(&mut self) {
        while let Some((key, value, _auto_gen)) = self.dirty.pop() {
            let query = "select count(1) as name_count from config where name = ?";
            if let Ok(conn) = self.conn.lock() {
                let mut stat = conn.prepare(query).unwrap();
                stat.bind((1, key.as_str())).unwrap();
                while let Ok(State::Row) = stat.next() {
                    let count = stat.read::<i64, _>("name_count").unwrap();
                    if count > 0 {
                        let update_query = format!("update config set value = '{}', auto_gen = {} where name = '{}'", value, _auto_gen, key);
                        conn.execute(update_query).unwrap();
                    } else {
                        let update_query = format!("insert into config (name, value, auto_gen) values ('{}', '{}', {});", key, value, _auto_gen);
                        conn.execute(update_query).unwrap();
                    }
                    break ;
                }
            }
        }
    }
}
