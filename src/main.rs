//! s3-sync: 一个简单的命令行工具，用于将本地目录同步到AWS S3存储桶
//! 
//! 该工具支持将本地目录的内容推送到S3存储桶，并确保远程目录与本地目录保持同步。
//! 它会比较文件的ETag来避免不必要的传输，并自动设置适当的Content-Type。

use anyhow::Result;
use clap::{Parser, Subcommand, Args};

mod commands;
mod utils;

/// 命令行参数定义
/// 
/// 使用clap crate定义命令行参数解析
#[derive(Parser)]
#[command(name = "s3-sync")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Push命令：将本地目录推送到S3存储桶
    /// 
    /// 该命令会扫描本地目录和远程S3存储桶，比较文件差异，
    /// 然后执行必要的上传和删除操作以保持同步。
    /// 存储桶通过环境变量AWS_BUCKET指定。
    Push(PushArgs),
}

/// Push命令参数
#[derive(Args)]
struct PushArgs {
    /// 本地目录路径
    /// 
    /// 需要同步到S3的本地目录的路径
    #[arg(index = 1)]
    local_dir: String,
    
    /// 远程S3路径
    /// 
    /// 格式为 "prefix"，指定S3存储桶内的前缀
    #[arg(index = 2)]
    remote_dir: String,
}

/// 主函数
/// 
/// 程序入口点，负责解析命令行参数并执行相应的操作
#[tokio::main]
async fn main() -> Result<()> {
    // 解析命令行参数
    let cli = Cli::parse();

    // 根据子命令执行相应操作
    match &cli.command {
        Commands::Push(args) => {
            // 执行push操作
            commands::push::push_files(&args.local_dir, &args.remote_dir).await?;
        }
    }

    Ok(())
}
