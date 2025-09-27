//! s3-sync: 一个简单的命令行工具，用于将本地目录同步到AWS S3存储桶
//! 
//! 该工具支持将本地目录的内容推送到S3存储桶，并确保远程目录与本地目录保持同步。
//! 它会比较文件的ETag来避免不必要的传输，并自动设置适当的Content-Type。

use clap::{Parser, Subcommand};
use std::path::Path;
use anyhow::Result;
use mime_guess::from_path;

// 引入s3_sync模块，包含与S3交互的核心功能
mod s3_sync;
use s3_sync::{get_s3_client, get_local_files, get_s3_files, generate_sync_operations, SyncOperation};

/// 命令行界面定义
/// 
/// 使用clap crate定义命令行参数解析
#[derive(Parser)]
#[command(name = "s3-sync")]
#[command(about = "A simple CLI tool for syncing files with S3", long_about = None)]
struct Cli {
    /// 定义可用的子命令
    #[command(subcommand)]
    command: Commands,
}

/// 可用的子命令枚举
/// 
/// 目前只支持Push命令，用于将本地目录推送到S3
#[derive(Subcommand)]
enum Commands {
    /// Push命令：将本地目录推送到S3存储桶
    /// 
    /// 该命令会扫描本地目录和远程S3存储桶，比较文件差异，
    /// 然后执行必要的上传和删除操作以保持同步。
    Push {
        /// 本地目录路径
        /// 
        /// 需要同步到S3的本地目录的路径
        #[arg(index = 1)]
        local_dir: String,
        
        /// 远程S3路径
        /// 
        /// 格式为 "bucket-name/prefix"，指定S3存储桶和可选的前缀
        #[arg(index = 2)]
        remote_dir: String,
    },
}

/// 主函数
/// 
/// 程序入口点，负责解析命令行参数并执行相应的操作
#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量（包括从.env文件）
    dotenvy::dotenv().ok();
    
    // 解析命令行参数
    let cli = Cli::parse();

    // 根据子命令执行相应操作
    match &cli.command {
        Commands::Push { local_dir, remote_dir } => {
            // 执行push操作
            push_files(local_dir, remote_dir).await?;
        }
    }
    
    Ok(())
}

/// Push文件到S3的主要函数
/// 
/// 该函数负责整个同步过程：
/// 1. 解析远程路径
/// 2. 获取S3客户端
/// 3. 扫描本地和远程文件
/// 4. 生成同步操作队列
/// 5. 执行操作队列
/// 
/// # Arguments
/// 
/// * `local_dir` - 本地目录路径
/// * `remote_dir` - 远程S3路径（格式：bucket/prefix）
async fn push_files(local_dir: &str, remote_dir: &str) -> Result<()> {
    // 解析远程目录为bucket和prefix
    // 例如："my-bucket/my-prefix" -> bucket="my-bucket", prefix="my-prefix"
    let parts: Vec<&str> = remote_dir.splitn(2, '/').collect();
    let bucket = parts[0];
    let prefix = if parts.len() > 1 { parts[1] } else { "" };
    
    // 确保prefix以'/'结尾（如果不是空的话）
    // 这样可以确保文件正确地放置在指定的前缀下
    let prefix = if !prefix.is_empty() && !prefix.ends_with('/') {
        format!("{}/", prefix)
    } else {
        prefix.to_string()
    };
    
    // 输出操作信息
    println!("Pushing {} to bucket: {}, prefix: {}", local_dir, bucket, prefix);
    
    // 获取S3客户端实例
    let client = get_s3_client().await;
    
    // 获取本地文件列表
    println!("Scanning local files...");
    let local_files = get_local_files(local_dir).await?;
    println!("Found {} local files", local_files.len());
    
    // 获取远程文件列表
    println!("Scanning remote files...");
    let remote_files = get_s3_files(&client, bucket, &prefix).await?;
    println!("Found {} remote files", remote_files.len());
    
    // 生成同步操作队列
    let operations = generate_sync_operations(&local_files, &remote_files);
    println!("Generated {} sync operations", operations.len());
    
    // 执行操作队列
    execute_operations(&client, local_dir, bucket, &prefix, operations).await?;
    
    // 输出完成信息
    println!("Push completed successfully!");
    Ok(())
}

/// 执行同步操作队列
/// 
/// 该函数按顺序执行所有同步操作（上传和删除）
/// 
/// # Arguments
/// 
/// * `client` - S3客户端实例
/// * `local_dir` - 本地目录路径
/// * `bucket` - S3存储桶名称
/// * `prefix` - S3前缀
/// * `operations` - 同步操作队列
async fn execute_operations(
    client: &aws_sdk_s3::Client,
    local_dir: &str,
    bucket: &str,
    prefix: &str,
    operations: Vec<SyncOperation>,
) -> Result<()> {
    // 遍历所有操作并执行
    for (index, operation) in operations.iter().enumerate() {
        // 输出当前操作进度
        println!("Executing operation {}/{}: {:?}", index + 1, operations.len(), operation);
        
        // 根据操作类型执行相应操作
        match operation {
            SyncOperation::Upload { local_path, remote_key } => {
                // 构建完整的本地文件路径
                let full_local_path = Path::new(local_dir).join(local_path);
                // 构建完整的远程键（key）
                let full_remote_key = format!("{}{}", prefix, remote_key);
                
                // 获取文件的内容类型
                let content_type = get_content_type(local_path);
                
                // 上传文件到S3
                client
                    .put_object()
                    .bucket(bucket)
                    .key(full_remote_key)
                    .body(aws_sdk_s3::primitives::ByteStream::from_path(&full_local_path).await?)
                    .content_type(content_type)
                    .send()
                    .await?;
                    
                // 输出上传成功信息
                println!("Uploaded: {}", local_path);
            }
            SyncOperation::Delete { remote_key } => {
                // 构建完整的远程键（key）
                let full_remote_key = format!("{}{}", prefix, remote_key);
                
                // 从S3删除文件
                client
                    .delete_object()
                    .bucket(bucket)
                    .key(full_remote_key)
                    .send()
                    .await?;
                    
                // 输出删除成功信息
                println!("Deleted: {}", remote_key);
            }
        }
    }
    
    Ok(())
}

/// 根据文件扩展名获取内容类型
/// 
/// 该函数使用mime_guess库根据文件扩展名自动检测MIME类型
/// 
/// # Arguments
/// 
/// * `file_path` - 文件路径
/// 
/// # Returns
/// 
/// * `String` - 内容类型字符串
fn get_content_type(file_path: &str) -> String {
    // 使用mime_guess库基于文件扩展名检测MIME类型
    from_path(file_path)
        .first_raw()
        .unwrap_or("binary/octet-stream")
        .to_string()
}
