use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{
    common::{config::{self, Config, CFG_PATH}, gen_uuid, utils}, 
    features::commands::{self, CommandData, FtPath, ApiCommand},
};

mod api;
mod cli_commands;
mod downloader;

pub fn main() {
    use cli_commands::Command as cli_enum;
    let cli = cli_commands::Cli::parse();
    let mut cli_config = Config::new(
        Some(utils::config_dir().to_string()), 
        Some(config::FILE_TUNNEL_CFG_CLIENT.to_string()),
        Some(config::FILE_TUNNEL_ENDPOINT_CLIENT.to_string()),
    );
    cli_config.init();
    if let Some(cmd) = &cli.command {
        match cmd {    
            cli_enum::SetLocalConfig { 
                share_key, 
                save_path, 
                tunnel_host, 
                password 
            } => {
                assert_eq!(Path::new(save_path).exists(), true, "please make sure it exists");
                cli_config.set(config::CFG_PATH.to_string(), save_path.clone(), None);
                cli_config.set(config::CFG_TUNNEL_HOST.to_string(), tunnel_host.clone(), None);
                cli_config.set(config::CFG_SHARE_KEY.to_string(), share_key.clone(), None);
                cli_config.set(config::CFG_CLIENT_KEY.to_string(), gen_uuid(), None);
                if let Some(password) = password {
                    cli_config.set(config::CFG_PASSWORD.to_string(), password.clone(), None);
                }
            },
            cli_enum::ShowLocalConfig { names } => {
                let names: Vec<String> = match names {
                    Some(names) => {
                        let new_names: Vec<String> = names.into_iter()
                                .filter(|name| cli_config.keys().contains(*name))
                                .map(|f| f.clone())
                                .collect();
                        if new_names.len() != names.len() {
                            eprintln!("some cfg name be ignored");
                        }
                        new_names
                    },
                    None => {
                        let new_names: Vec<String> = cli_config.keys().iter()
                                        .filter(|f| (**f) != config::CFG_PASSWORD.to_string())
                                        .map(|f| f.clone())
                                        .collect();
                        new_names
                    }
                };
                let _: Vec<_> = cli_config.get_keys(Some(names)).iter().map(|(key, value)| {
                    println!("{key}: {value}");
                }).collect();
            }
            cli_enum::ReadServerConfig {  } => {
                let cmd = ApiCommand {
                    version: 1,
                    command: commands::Command::ReadConfig {  }
                };
                match api::do_http_request(&mut cli_config, &cmd) {
                    Ok(result) => println!("server config: {}", String::from_iter(result.iter())),
                    Err(err) => eprintln!("error: {}", err.to_string()),
                }
            },
            cli_enum::ReadDirItem { dir_path, take_size, skip_size, format, download } => {
                match downloader::download(
                    &mut cli_config,
                    PathBuf::from(dir_path),
                    *take_size,
                    *skip_size,
                    *download,
                ) {
                    Ok(()) => {},
                    Err(e) => eprint!("read dir item failed, {}", e)
                }
            },
            cli_enum::ReadFileInfo { file_path } => {
                let root_path = cli_config.get_key(CFG_PATH.to_string()).unwrap_or("/".to_string());
                let cmd = ApiCommand {
                    version: 1,
                    command: commands::Command::ReadFileInfo { 
                        file_path: FtPath::new_relative(root_path, file_path.clone()),
                    }
                };

                match api::do_http_request_data(&mut cli_config, &cmd) {
                    Ok(message) => match message.data {
                        CommandData::DownloadFile { data, data_size } => {

                        },
                        _ => {},
                    },
                    Err(e) => eprintln!("request error: {}", e)
                }
            },
            cli_enum::DownloadFile { file_path, block_size, block_idx } => {
                let root_path = cli_config.get_key(CFG_PATH.to_string()).unwrap_or("/".to_string());
                let cmd = ApiCommand {
                    version: 1,
                    command: commands::Command::DownloadFile { 
                        file_path: FtPath::new_relative(root_path, file_path.clone()),
                        block_idx: *block_idx,
                        block_size: *block_size,
                    }
                };

                if let Ok(result) = api::do_http_request(&mut cli_config, &cmd) {
                    println!("result: {}", String::from_iter(result.iter()));
                }
            }
        }
    }
}
