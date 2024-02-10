use std::fs::{self, ReadDir};
use std::{sync::mpsc::channel, thread};

use clap::Parser;
use websocket::{Message, OwnedMessage};
use crate::common::{self, CommomResult};
use crate::features::commands::{ApiCommand, CommandData, CommandMessage, DirItem};
use crate::features::server::command_handler::send_message_text;

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
            headers.append_raw("X-Server-Key", share_key_chars);
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
                    match parse_message(message.clone()) {
                        Ok((client_key, message)) => {
                            match message {
                                ParsedMessage::Text(next_data) => {
                                    let cmd: Result<ApiCommand, serde_json::Error> = serde_json::from_str(&next_data);
                                    let root_path = config.get_key(config::CFG_PATH.to_string()).unwrap();
                                    match cmd {
                                        Ok(cmd) => {
                                            if let Err(e) = command_handler::handler(
                                                &tx1, root_path.as_str(), 
                                                client_key.as_str(),
                                                &cmd
                                            ) {
                                                let message = CommandMessage {
                                                    version: cmd.version,
                                                    status: 500,
                                                    data: CommandData::Error { message: e.to_string() },
                                                };
                                                send_message_text(
                                                    &tx1, 
                                                    client_key.as_str(),
                                                    message
                                                ).unwrap();
                                            }
                                        },

                                        Err(e) => {
                                            println!("cmd parse failed, err:{e}, txt: {next_data}");
                                            let message = CommandMessage {
                                                version: 0,
                                                status: 404,
                                                data: CommandData::Error { 
                                                    message: format!("data parse error, message: {}", e.to_string()),
                                                },
                                            };
                                            send_message_text(
                                                &tx,
                                                client_key.as_str(),
                                                message
                                            ).unwrap();
                                        }
                                    }
                                },
                                ParsedMessage::Binary(_) => eprintln!("todo next"),
                                ParsedMessage::Raw(_message) => (),
                            }
                        },
                        Err(e) => {
                            eprintln!("message: {:#?} {}", message, e)
                        },
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

#[allow(dead_code)]
enum ParsedMessage {
    Text(String),
    Binary(Vec<u8>),
    Raw(OwnedMessage),
}

fn parse_message(message: OwnedMessage) -> CommomResult<(String, ParsedMessage)> {
    match message {
        OwnedMessage::Binary(bin) => {
            let client_key_size = bin[0] as usize;
            let client_key = String::from_iter(bin[1..(client_key_size+1)].iter().map(|b| *b as char).collect::<Vec<char>>().iter());
            Ok((client_key, ParsedMessage::Binary(bin[(client_key_size + 1)..].to_vec())))
        },
        OwnedMessage::Text(text) => {
            let (_cks, text) = text.split_at(3);
            let client_key_size = _cks.to_string().parse::<usize>()?;
            let (client_key, message) = text.split_at(client_key_size);
            Ok((client_key.to_string(), ParsedMessage::Text(message.to_string())))
        },
        _ => Ok(("".to_string(), ParsedMessage::Raw(message.clone())))
    }
}
