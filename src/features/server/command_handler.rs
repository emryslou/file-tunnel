use std::{cmp::min, fs, io::{Read as _, Seek as _, SeekFrom}};

use crate::{common::CommomResult, features::commands::{self, ApiCommand, CommandData, CommandMessage, DirItem}};
use websocket::OwnedMessage;
use std::sync::mpsc::Sender;

use super::{build_item, dir_iter};

pub fn handler(tx1: &Sender<OwnedMessage>, root_path: &str, client_key: &str, cmd: &ApiCommand) -> CommomResult<()> {
    match &cmd.command {
        commands::Command::ReadConfig {} => {
            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadConfig {
                    path: "/".to_string(),
                },
            };
            send_message_text(tx1, client_key, message)?;
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
            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadDirItem {
                    items: dir_items,
                    total: total,
                    taked_size: min(*take_size + *skip_size, total),
                },
            };
            send_message_text(tx1, client_key, message)?;
        }
        commands::Command::ReadFileInfo { file_path } => {
            let mut file_path = file_path.clone();
            let org_root_path = file_path.root_path().clone();
            file_path.reset_root(&root_path);
            log::debug!("read file info: {}", file_path.full_path());
            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::ReadFileInfo {
                    item: build_item(&file_path.full_path(), &root_path, &org_root_path),
                },
            };
            send_message_text(tx1, client_key, message)?;
        }
        commands::Command::DownloadFile {
            file_path,
            block_idx,
            block_size,
        } => {
            let mut file_path = file_path.clone();
            file_path.reset_root(&root_path);
            let mut f = fs::File::open(file_path.full_path())?;
            if *block_idx > 0 {
                let _seek_size = f
                    .seek(SeekFrom::Start(((*block_idx) * (*block_size)) as u64))?;
            }
            let mut buffer: Vec<u8> = vec![0u8; *block_size];
            let real_size = f.read(&mut buffer)?;

            let message = CommandMessage {
                version: cmd.version,
                status: 0,
                data: CommandData::DownloadFile {
                    data: buffer[..real_size].to_vec(),
                    data_size: real_size,
                },
            };
            log::info!("file: {} ({}*{})", file_path, block_idx, block_size);
            send_message_text(tx1, client_key, message)?;
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
            send_message_text(tx1, client_key, message)?;
        }
        _ => {
            println!("cannot support comand:{cmd:#?}");
        }
    }
    Ok(())
}

pub fn send_message_text(tx: &Sender<OwnedMessage>, client_key: &str, message: CommandMessage) -> CommomResult<()>{
    let message = format!(
        "{:03}{}{}\0\0\0\0",
        &client_key.len(),
        &client_key,
        serde_json::to_string(&message)?
    );
    tx.send(OwnedMessage::Text(message))?;
    Ok(())
}

pub fn send_message_binary(tx: &Sender<OwnedMessage>, client_key: &str, message: Vec<u8>) -> CommomResult<()>{
    let mut send_message: Vec<u8> = vec![client_key.len() as u8];
    send_message.extend(client_key.as_bytes().to_vec());
    send_message.extend(message.iter());
    send_message.extend(vec![0u8; 4]);
    tx.send(OwnedMessage::Binary(message))?;
    Ok(())
}
