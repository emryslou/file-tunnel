use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>
}

#[derive(Subcommand, Debug)]
pub enum Command {
    
    // 1. 设置配置【share_key, save_path, tunnel_host, password】
    SetLocalConfig {
        #[arg(long)]
        share_key: String,

        #[arg(long)]
        save_path: String,

        #[arg(long)]
        tunnel_host: String,

        #[arg(long)]
        password: Option<String>,
    },

    // 2. 显示配置
    ShowLocalConfig {
        #[arg(long)]
        names: Option<Vec<String>>
    },

    // 3. 读取 server 端配置
    ReadServerConfig {},

    // 4. 获取 目录 信息
    ReadDirItem {
        #[arg(long)]
        dir_path: String,

        #[arg(long, default_value_t=10)]
        take_size: usize,

        #[arg(long, default_value_t=0)]
        skip_size: usize,

        #[arg(long, default_value="text")]
        format: String,

        #[arg(long, default_value_t=false)]
        download: bool,
    },

    // 5. 获取 文件 信息
    ReadFileInfo {
        #[arg(long)]
        file_path: String,
    },

    // 6. 下载文件
    DownloadFile {
        #[arg(long)]
        file_path: String,

        #[arg(long, default_value_t=1024)]
        block_size: usize,

        #[arg(long, default_value_t=0)]
        block_idx: usize,
    }
}
