use std::cmp::min;
use std::fs::{self, ReadDir};
use std::{sync::mpsc::channel, thread};

use std::io::{Read, Seek, SeekFrom};
use clap::Parser;
use websocket::{Message, OwnedMessage};
use crate::common;
use crate::features::commands::{self, CommandData, CommandMessage, DirItem, WebSocketCommand};

use super::commands::FtPath;

mod cli_command;


pub fn main() {
    use cli_command::Commands;
    use common::{config, utils};
    let cli = cli_command::Cli::parse();
    let mut config = config::Config::new(
        Some(utils::config_dir()), 
        Some(config::FILE_TUNNEL_CFG_SERVER.to_string()),
        None
    );
    config.init();
    match &cli.command {
        Commands::SetConfig { path, tunnel_host, password } => {
            config.set(config::CFG_PATH.to_string(), path.clone(), None);
            config.set(config::CFG_TUNNEL_HOST.to_string(), tunnel_host.clone(), None);
            config.set(config::CFG_SHARE_KEY.to_string(), common::gen_uuid(), None);
            match password {
                Some(password) => {
                    config.set(config::CFG_PASSWORD.to_string(), password.clone(), None);
                },
                None => {
                    config.set(config::CFG_PASSWORD.to_string(), common::gen_password(16), Some(1));
                },
            };
        },
        Commands::ShowConfig { names } => {
            let names: Vec<String> = match names {
                Some(names) => {
                    let new_names: Vec<String> = names.into_iter()
                            .filter(|name| config.keys().contains(*name))
                            .map(|f| f.clone())
                            .collect();
                    if new_names.len() != names.len() {
                        eprintln!("some cfg name be ignored");
                    }
                    new_names
                },
                None => {
                    let new_names: Vec<String> = config.keys().iter()
                                    .filter(|f| (**f) != config::CFG_PASSWORD.to_string())
                                    .map(|f| f.clone())
                                    .collect();
                    new_names
                }
            };
            let _: Vec<_> = config.get_keys(Some(names)).iter().map(|(key, value)| {
                println!("{key}: {value}");
            }).collect();
        },
        Commands::Start {  } => {
            let config_values= config.get_keys_to_map(Some(vec![
                config::CFG_SHARE_KEY.to_string(),
                config::CFG_TUNNEL_HOST.to_string()
            ]));

            let share_key_chars: Vec<u8> = config_values.get(&config::CFG_SHARE_KEY.to_string()).unwrap().as_bytes().into_iter().map(|c| *c).collect();
            let tunnel_host = config_values.get(&config::CFG_TUNNEL_HOST.to_string()).unwrap();
            let mut headers = websocket::header::Headers::new();
            headers.append_raw("X-Share-Key", share_key_chars);
            let client = websocket::ClientBuilder::new(
                    format!("ws://{}/tunnel/v1/server/ws", tunnel_host).as_str()
                ).unwrap()
                .custom_headers(&headers)
                .add_protocol("rust-websocket")
                .connect_insecure()
                .unwrap();

            let (mut recver, mut sender) = client.split().unwrap();

            let (tx, rx) = channel();
            let tx1 = tx.clone();
            let send_loop = thread::spawn(move|| {
                loop {
                    let message = match rx.recv() {
                        Ok(m) => m,
                        Err(e) => {
                            println!("Error: {}", e);
                            break ;
                        }
                    };

                    match message {
                        OwnedMessage::Close(_) => {
                            println!("msg: {:?}", message);
                            let _ = sender.send_message(&message);
                            return ;
                        }
                        _ => {}
                    };

                    match sender.send_message(&message) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("Send Loop: {:?}", e);
                            let _ = sender.send_message(&Message::close());
                            return ;
                        }
                    }
                }
            });
            let recv_loop = thread::spawn(move|| {
                for message in recver.incoming_messages() {
                    let message = match message {
                        Ok(m) => m,
                        Err(e) => {
                            println!("Receive loop: {:?}", e);
                            println!(":quit\n");
                            let _ = tx1.send(OwnedMessage::Close(None));
                            break ;
                        }
                    };

                    match message {
                        OwnedMessage::Close(_) => {
                            println!("should be quit");
                            let _ = tx1.send(OwnedMessage::Close(None));
                            break ;
                        },
                        OwnedMessage::Ping(data) | OwnedMessage::Pong(data) => {
                            println!(">: {:?}", data);
                        },
                        OwnedMessage::Binary(bin) => {
                            println!(">bin: {:?}", bin);
                        }
                        OwnedMessage::Text(txt) => {
                            if txt.len() > 50 {
                                println!("txt: {}", &txt[..50]);
                            } else {
                                println!("txt: {}", txt);
                            }
                            let cmd: Result<WebSocketCommand, serde_json::Error> = serde_json::from_str(&txt);
                            let root_path = config.get_key(config::CFG_PATH.to_string()).unwrap();
                            match cmd {
                                Ok(cmd) => {
                                    match &cmd.command {
                                        commands::Command::ReadConfig {  } => {
                                            let data = CommandMessage {
                                                version: cmd.version,
                                                status: 0,
                                                data: CommandData::ReadConfig { path: "/".to_string() }
                                            };
                                            let _ = tx1.send(OwnedMessage::Text(serde_json::to_string(&data).unwrap()));
                                        }
                                        commands::Command::ReadDirItem { dir_path, take_size, skip_size } => {
                                            let mut dir_path = dir_path.clone();
                                            let org_root_path = dir_path.root_path().clone();
                                            dir_path.reset_root(&root_path);
                                            let total = dir_iter(&dir_path).count();
                                            let dir_items: Vec<DirItem> = dir_iter(&dir_path)
                                                .skip(*skip_size).take(*take_size)
                                                .map(|dir| {
                                                    build_item(
                                                            dir.unwrap().path().to_str().unwrap(),
                                                            &root_path,
                                                            &org_root_path
                                                        )
                                                    }
                                                ).collect();
                                            let data = CommandMessage {
                                                version: cmd.version,
                                                status: 0,
                                                data: CommandData::ReadDirItem {
                                                    items: dir_items,
                                                    total: total,
                                                    taked_size: min(*take_size + *skip_size, total),
                                                }
                                            };
                                            
                                            let _ = tx1.send(OwnedMessage::Text(serde_json::to_string(&data).unwrap()));
                                        }
                                        commands::Command::ReadFileInfo { file_path } => {
                                            let mut file_path = file_path.clone();
                                            let org_root_path = file_path.root_path().clone();
                                            file_path.reset_root(&root_path);

                                            let message = CommandMessage {
                                                version: cmd.version,
                                                status: 0,
                                                data: CommandData::ReadFileInfo {
                                                    item: build_item(&file_path.full_path(), &root_path, &org_root_path)
                                                },
                                            };
                                            
                                            let _ = tx1.send(OwnedMessage::Text(serde_json::to_string(&message).unwrap()));
                                        }
                                        commands::Command::DownloadFile { 
                                            file_path, block_idx, block_size
                                        } => {
                                            let mut file_path = file_path.clone();
                                            file_path.reset_root(&root_path);
                                            let mut f = fs::File::open(file_path.full_path()).unwrap();
                                            if *block_idx > 0 {
                                                let _seek_size = f.seek(SeekFrom::Start(((*block_idx) * (*block_size)) as u64)).unwrap();
                                            }
                                            let mut buffer: Vec<u8> = vec![0u8; *block_size];
                                            let real_size = f.read(&mut buffer).unwrap();

                                            let data = CommandMessage {
                                                version: cmd.version,
                                                status: 0,
                                                data: CommandData::DownloadFile { 
                                                    data: buffer[..real_size].to_vec(),
                                                    data_size: real_size,
                                                }
                                            };
                                            let _ = tx1.send(OwnedMessage::Text(serde_json::to_string(&data).unwrap()));
                                        },
                                        commands::Command::ReadPathInfo { path, take_size, skip_size } => {
                                            let mut path = path.clone();
                                            let org_root_path = path.root_path().clone();
                                            path.reset_root(&root_path);
                                            let res_data = match fs::metadata(path.full_path()) {
                                                Ok(meta) => {
                                                    if meta.is_file() {
                                                        CommandData::ReadFileInfo { 
                                                            item: build_item(
                                                                &path.full_path(),
                                                                &root_path,
                                                                &org_root_path
                                                            )
                                                        }
                                                    } else if meta.is_dir() {
                                                        let total_count = dir_iter(&path).count();
                                                        CommandData::ReadDirItem { 
                                                            items: dir_iter(&path).skip(*skip_size)
                                                                .take(*take_size)
                                                                .map(|dir| {
                                                                    build_item(
                                                                        dir.unwrap().path().to_str().unwrap(),
                                                                        &root_path,
                                                                        &org_root_path
                                                                    )
                                                                })
                                                                .collect(),
                                                            total: total_count,
                                                            taked_size: min(*take_size + *skip_size, total_count)
                                                        }
                                                    } else {
                                                        CommandData::Error { message: "todo next".to_string() }
                                                    }
                                                },
                                                Err(err) => CommandData::Error { message: err.to_string() }
                                            };

                                            let res = CommandMessage {
                                                version: cmd.version,
                                                status: 0,
                                                data: res_data,
                                            };
                                            let _ = tx1.send(OwnedMessage::Text(serde_json::to_string(&res).unwrap()));
                                        },
                                        _ => {
                                            println!("cannot support comand:{cmd:#?}");
                                        }
                                    }
                                    let _ = tx1.send(OwnedMessage::Binary(vec![0u8; 4])).unwrap();
                                },
                                Err(e) => {
                                    println!("cmd parse failed, err:{e}, txt: {txt}");
                                    let _ = tx1.send(OwnedMessage::Binary(vec![0u8; 4])).unwrap();
                                }
                            }
                        }
                    }
                }
            });

            let _ = send_loop.join();
            let _ = recv_loop.join();
            println!("Exit ...");
        },
        Commands::Stop {} => {
        }
        Commands::Restart {  } => {}
    }
}

fn build_item(path: &str, root: &str, org_root: &str) -> DirItem {
    let mut item = DirItem::from(path);
    item.path.replace_root_path(&root);
    item.path.reset_root(&org_root);
    item
}

fn dir_iter(ft_path: &FtPath) -> ReadDir {
    fs::read_dir(ft_path.full_path()).unwrap()
}
