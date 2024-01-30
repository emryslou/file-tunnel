use std::cmp::min;
use std::fs::{self, ReadDir};
use std::{sync::mpsc::channel, thread};

use std::io::{Read, Seek, SeekFrom};
use clap::Parser;
use websocket::{Message, OwnedMessage};
use crate::common;
use crate::features::commands::{self, CommandData, CommandMessage, DirItem, ApiCommand};

use super::commands::FtPath;

mod cli_command;
mod command_handler;


pub fn main() {
    use cli_command::Commands;
    use common::{config, utils};
    let cli = cli_command::Cli::parse();
    let mut config = config::Config::new(
        Some(utils::config_dir()), 
        Some(config::FILE_TUNNEL_CFG_SERVER.to_string()),
        Some(config::FILE_TUNNEL_ENDPOINT_SERVER.to_string()),
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
                            match txt.split_once(":") {
                                Some((client_key_size_str, next_data)) => {
                                    let client_key_size: usize = client_key_size_str.to_string().parse().unwrap();
                                    let client_key = next_data[..client_key_size].to_string();
                                    let next_data = next_data[client_key_size..].to_string();
                                    let cmd: Result<ApiCommand, serde_json::Error> = serde_json::from_str(&next_data);
                                    let root_path = config.get_key(config::CFG_PATH.to_string()).unwrap();
                                    match cmd {
                                        Ok(cmd) => {
                                            command_handler::handler(&tx1, root_path.as_str(), client_key.as_str(), &cmd);
                                            let mut msg = vec![client_key_size as u8];
                                            let chars: Vec<u8> = client_key.chars().map(|c| c as u8).collect();
                                            msg.extend(chars.iter());
                                            msg.extend(vec![0u8;4]);
                                            eprintln!("bin_msg: {:?}", msg);
                                            let _ = tx1.send(OwnedMessage::Binary(msg)).unwrap();
                                        },
                                        Err(e) => {
                                            let mut msg = vec![client_key_size as u8];
                                            let chars: Vec<u8> = client_key.chars().map(|c| c as u8).collect();
                                            msg.extend(chars.iter());
                                            msg.extend(vec![0u8;4]);
                                            eprintln!("bin_msg 111: {:?}", msg);
                                            println!("cmd parse failed, err:{e}, txt: {txt}");
                                            let _ = tx1.send(OwnedMessage::Binary(vec![0u8; 4])).unwrap();
                                        }
                                    }
                                },
                                None => {}
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
