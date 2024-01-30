use sha256::try_digest;
use std::{fs, io::{Seek, SeekFrom, Write}, path::PathBuf};

use crate::{
    common::{config::{Config, CFG_PATH}, utils, CommomResult},
    features::commands::{Command, CommandData, DirItem, DirItemInfo, FtPath, ApiCommand}
};

use super::api;


pub fn download(cli_config: &mut Config, path: PathBuf, take_size: usize, skip_size: usize, download: bool) -> CommomResult<()> {
    let root_path = cli_config.get_key(CFG_PATH.to_string()).unwrap();
    let cmd = ApiCommand {
        version: 1,
        command: Command::ReadPathInfo {
            path: FtPath::new_absolute(root_path, path.to_str().unwrap().to_string()),
            take_size, skip_size,
        }
    };
    let message = api::do_http_request_data(cli_config, &cmd)?;
    match message.data {
        CommandData::ReadDirItem { items, total, taked_size } => {
            let _: Vec<_> = items.iter().map(|dir| {
                let stat = if dir.path.exists() { "L" } else { "R" };
                let mut item_type = "";
                let mut size = "".to_string();
                match &dir.info {
                    DirItemInfo::File { modified_at, created_at, file_size, chksum} => {
                        item_type = "-";
                        size = utils::format_size(*file_size);
                    },
                    DirItemInfo::Dir { modified_at, created_at, item_count } => {
                        item_type = "d";
                        size = format!("{}", item_count);
                    }
                }
                if download && !dir.path.exists() {
                    downloader(cli_config, dir).unwrap();
                }
                println!("{}--------- {} {} {}", item_type, size, dir.path.full_path(), stat);
            }).collect();
            println!("info: {}/{}", taked_size, total);
        },
        CommandData::ReadFileInfo { item } => {
            downloader(cli_config, &item)?
        },
        _data => eprintln!("response error")
    }
    Ok(())
}

fn downloader(cli_config: &mut Config, item: &DirItem) -> CommomResult<()> {
    match &item.info {
        DirItemInfo::File { modified_at, created_at, file_size, chksum } => {
            let block_size = 1 << 16;
            let mut block_count = file_size / block_size;
            if block_count * block_size < *file_size {
                block_count += 1;
            }
            let mut file = if !item.path().exists() {
                fs::File::create(item.path().full_path())?
            } else {
                fs::File::open(item.path().full_path())?
            };
            file.rewind()?;

            let mut downloaded_size: u64 = 0;
            let mut block_idx = 0;
            let max_err_retry_times = 3;
            let mut err_times = max_err_retry_times;
            while block_idx < block_count {
                let cmd = ApiCommand {
                    version: 1,
                    command: Command::DownloadFile {
                        file_path: item.path.clone(),
                        block_idx: block_idx as usize,
                        block_size: block_size as usize,
                    }
                };
                let message = api::do_http_request_data(cli_config, &cmd)?;

                match message.data {
                    CommandData::DownloadFile { data, data_size } => {
                        if data_size > 0 {
                            file.seek(SeekFrom::Start(downloaded_size))?;
                            let wsize = file.write(&data)?;
                            if wsize != data_size {
                                panic!("write file error");
                            }
                            downloaded_size += data_size as u64;
                            block_idx += 1;
                            let percent: f64 = (downloaded_size as f64 / *file_size as f64) * (100 as f64);
                            let downloaded_size_er = utils::format_size(downloaded_size);
                            let total_size_er = utils::format_size(*file_size);
                            print!("{:<50}: [{}/{},{:>6}]\r", item.path().full_path(), downloaded_size_er, total_size_er, format!("{:.2}%", percent));
                        }
                        if data_size < block_size as usize {
                            let local_chksum = try_digest(item.path().full_path())?;
                            if local_chksum == *chksum {
                                break ;
                            } else {
                                err_times -= 0;
                                if err_times > 0 {
                                    eprintln!("sha256sum valid faild, and will retry it {:?} <=>{:?}", local_chksum, &chksum);
                                    block_idx = 0;
                                    downloaded_size = 0;
                                } else {
                                    panic!("sha256sum valid faild too many times");
                                }
                            }
                        }
                    },
                    _ => panic!("unexpected message")
                }
            }
            
            Ok(())
        },
        DirItemInfo::Dir { modified_at, created_at, item_count } => {
            fs::create_dir(item.path.full_path())?;
            Ok(())
        }
    }
}
