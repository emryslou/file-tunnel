use std::{cmp::min, fs, io::{Read as _, Seek as _, SeekFrom}};

use crate::features::commands::{self, ApiCommand, CommandData, CommandMessage, DirItem};
use websocket::{Message, OwnedMessage};
use std::sync::mpsc::Sender;

use super::{build_item, dir_iter};

pub fn handler(tx1: &Sender<OwnedMessage>, root_path: &str, client_key: &str, cmd: &ApiCommand) {
    match &cmd.command {
        commands::Command::ReadConfig {} => {
            let data = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadConfig {
                    path: "/".to_string(),
                },
            };
            let msg = format!(
                "{}:{}{}",
                &client_key.len(),
                &client_key,
                serde_json::to_string(&data).unwrap()
            );
            let _ = tx1.send(OwnedMessage::Text(msg));
        }
        commands::Command::ReadDirItem {
            dir_path,
            take_size,
            skip_size,
        } => {
            let mut dir_path = dir_path.clone();
            let org_root_path = dir_path.root_path().clone();
            dir_path.reset_root(&root_path);
            let total = dir_iter(&dir_path).count();
            let dir_items: Vec<DirItem> = dir_iter(&dir_path)
                .skip(*skip_size)
                .take(*take_size)
                .map(|dir| {
                    build_item(
                        dir.unwrap().path().to_str().unwrap(),
                        &root_path,
                        &org_root_path,
                    )
                })
                .collect();
            let data = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadDirItem {
                    items: dir_items,
                    total: total,
                    taked_size: min(*take_size + *skip_size, total),
                },
            };
            let msg = format!(
                "{}:{}{}",
                &client_key.len(),
                &client_key,
                serde_json::to_string(&data).unwrap()
            );
            let _ = tx1.send(OwnedMessage::Text(msg));
        }
        commands::Command::ReadFileInfo { file_path } => {
            let mut file_path = file_path.clone();
            let org_root_path = file_path.root_path().clone();
            file_path.reset_root(&root_path);

            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadFileInfo {
                    item: build_item(&file_path.full_path(), &root_path, &org_root_path),
                },
            };

            let msg = format!(
                "{}:{}{}",
                &client_key.len(),
                &client_key,
                serde_json::to_string(&message).unwrap()
            );
            println!("send msg: {}", msg);
            let _ = tx1.send(OwnedMessage::Text(msg));
        }
        commands::Command::DownloadFile {
            file_path,
            block_idx,
            block_size,
        } => {
            let mut file_path = file_path.clone();
            file_path.reset_root(&root_path);
            let mut f = fs::File::open(file_path.full_path()).unwrap();
            if *block_idx > 0 {
                let _seek_size = f
                    .seek(SeekFrom::Start(((*block_idx) * (*block_size)) as u64))
                    .unwrap();
            }
            let mut buffer: Vec<u8> = vec![0u8; *block_size];
            let real_size = f.read(&mut buffer).unwrap();

            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::DownloadFile {
                    data: buffer[..real_size].to_vec(),
                    data_size: real_size,
                },
            };
            let msg = format!(
                "{}:{}{}",
                &client_key.len(),
                &client_key,
                serde_json::to_string(&message).unwrap()
            );
            let _ = tx1.send(OwnedMessage::Text(msg));
        }
        commands::Command::ReadPathInfo {
            path,
            take_size,
            skip_size,
        } => {
            let mut path = path.clone();
            let org_root_path = path.root_path().clone();
            path.reset_root(&root_path);
            let res_data = match fs::metadata(path.full_path()) {
                Ok(meta) => {
                    if meta.is_file() {
                        CommandData::ReadFileInfo {
                            item: build_item(&path.full_path(), &root_path, &org_root_path),
                        }
                    } else if meta.is_dir() {
                        let total_count = dir_iter(&path).count();
                        CommandData::ReadDirItem {
                            items: dir_iter(&path)
                                .skip(*skip_size)
                                .take(*take_size)
                                .map(|dir| {
                                    build_item(
                                        dir.unwrap().path().to_str().unwrap(),
                                        &root_path,
                                        &org_root_path,
                                    )
                                })
                                .collect(),
                            total: total_count,
                            taked_size: min(*take_size + *skip_size, total_count),
                        }
                    } else {
                        CommandData::Error {
                            message: "todo next".to_string(),
                        }
                    }
                }
                Err(err) => CommandData::Error {
                    message: err.to_string(),
                },
            };

            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: res_data,
            };
            let msg = format!(
                "{}:{}{}",
                &client_key.len(),
                &client_key,
                serde_json::to_string(&message).unwrap()
            );
            let _ = tx1.send(OwnedMessage::Text(msg));
        }
        _ => {
            println!("cannot support comand:{cmd:#?}");
        }
    }
}
