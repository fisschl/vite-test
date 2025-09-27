//! Push命令核心功能模块
//! 
//! 该模块包含了Push命令的核心功能函数

use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use md5;
use crate::utils::mime::get_content_type;
use std::time::SystemTime;
use std::env as std_env;

/// 获取AWS S3客户端
/// 
/// 该函数会尝试从环境变量加载AWS凭证，如果失败则使用默认凭证链
/// 
/// # Returns
/// 
/// * `Client` - 配置好的S3客户端实例
pub async fn get_s3_client() -> Client {
    // 加载环境变量（包括从.env文件）
    dotenvy::dotenv().ok();

    // 尝试从环境变量获取AWS配置
    let config = if let (Ok(access_key), Ok(secret_key), Ok(region)) = (
        std_env::var("AWS_ACCESS_KEY_ID"),
        std_env::var("AWS_SECRET_ACCESS_KEY"),
        std_env::var("AWS_REGION"),
    ) {
        // 如果有自定义端点URL，使用它
        let mut config_builder = aws_config::from_env()
            .behavior_version(BehaviorVersion::latest())
            .region(aws_config::Region::new(region));
            
        // 如果设置了自定义端点URL，配置它
        if let Ok(endpoint_url) = std_env::var("AWS_ENDPOINT_URL") {
            config_builder = config_builder.endpoint_url(endpoint_url);
        }
        
        // 使用环境变量中的凭证创建配置
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None::<SystemTime>,
            "env",
        );
        
        // 使用指定的凭证和区域创建AWS配置
        config_builder
            .credentials_provider(credentials)
            .load()
            .await
    } else {
        // 如果环境变量不可用，使用默认凭证链
        aws_config::load_defaults(BehaviorVersion::latest()).await
    };
    
    // 创建并返回S3客户端
    Client::new(&config)
}

/// 获取本地目录中的所有文件
/// 
/// 递归扫描指定目录，返回所有文件的信息（路径、大小等）
/// 
/// # Arguments
/// 
/// * `local_dir` - 要扫描的本地目录路径
/// 
/// # Returns
/// 
/// * `Result<HashMap<String, FileInfo>>` - 文件信息映射或错误
pub async fn get_local_files(local_dir: &str) -> anyhow::Result<HashMap<String, FileInfo>> {
    // 创建文件映射，用于存储文件信息
    let mut files = HashMap::new();
    
    // 使用栈来递归遍历目录（避免递归函数调用）
    let mut stack = vec![local_dir.to_string()];
    
    // 当栈不为空时继续遍历
    while let Some(current_dir) = stack.pop() {
        // 读取当前目录的内容
        let mut entries = fs::read_dir(&current_dir).await?;
        
        // 遍历目录中的每个条目
        while let Some(entry) = entries.next_entry().await? {
            // 获取条目的路径和元数据
            let path = entry.path();
            let metadata = entry.metadata().await?;
            
            // 如果是目录，将其添加到栈中以供后续遍历
            if metadata.is_dir() {
                stack.push(path.to_string_lossy().to_string());
            } else {
                // 如果是文件，计算其相对路径并添加到文件列表
                let relative_path = path.strip_prefix(local_dir)?.to_string_lossy().to_string();
                // 规范化路径分隔符为正斜杠（确保跨平台兼容性）
                let relative_path = relative_path.replace('\\', "/");
                
                // 创建文件信息结构体
                let file_info = FileInfo {
                    etag: calculate_local_etag(&path).await?,
                };
                
                // 将文件信息添加到映射中
                files.insert(relative_path, file_info);
            }
        }
    }
    
    // 返回文件映射
    Ok(files)
}

/// 计算本地文件的ETag
/// 
/// 通过计算文件内容的MD5哈希来生成ETag，用于与S3中的ETag进行比较
/// 
/// # Arguments
/// 
/// * `file_path` - 文件路径
/// 
/// # Returns
/// 
/// * `Result<String>` - 文件的ETag或错误
async fn calculate_local_etag(file_path: &Path) -> anyhow::Result<String> {
    // 读取文件内容
    let content = fs::read(file_path).await?;
    // 计算MD5哈希
    let digest = md5::compute(&content);
    // 将哈希转换为十六进制字符串并返回
    Ok(format!("{:x}", digest))
}

/// 获取S3存储桶中的所有文件
/// 
/// 列出指定存储桶和前缀下的所有文件，并返回它们的信息
/// 
/// # Arguments
/// 
/// * `client` - S3客户端实例
/// * `bucket` - S3存储桶名称
/// * `prefix` - 文件前缀（可选）
/// 
/// # Returns
/// 
/// * `Result<HashMap<String, FileInfo>>` - 文件信息映射或错误
pub async fn get_s3_files(client: &Client, bucket: &str, prefix: &str) -> anyhow::Result<HashMap<String, FileInfo>> {
    // 创建文件映射，用于存储文件信息
    let mut files = HashMap::new();
    // 用于分页的延续令牌
    let mut continuation_token = None;
    
    // 循环处理分页结果
    loop {
        // 构建列表对象请求
        let mut request = client.list_objects_v2()
            .bucket(bucket)
            .prefix(prefix);
            
        // 如果有延续令牌，添加到请求中
        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }
        
        // 发送请求并获取响应
        let response = request.send().await?;
        
        // 处理响应中的文件列表
        if let Some(contents) = response.contents {
            for object in contents {
                // 如果对象包含必要的信息（键名、ETag、大小）
                if let (Some(key), Some(etag), Some(_size)) = (&object.key, &object.e_tag, &object.size) {
                    // 移除前缀以获得相对路径
                    let relative_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        key.strip_prefix(prefix).unwrap_or(key).to_string()
                    };
                    
                    // 移除开头的斜杠（如果存在）
                    let relative_key = relative_key.trim_start_matches('/').to_string();
                    
                    // 创建文件信息结构体
                    let file_info = FileInfo {
                        etag: etag.clone(),
                    };
                    
                    // 将文件信息添加到映射中
                    files.insert(relative_key, file_info);
                }
            }
        }
        
        // 检查是否还有更多页面
        if response.is_truncated.unwrap_or(false) {
            // 如果有更多页面，保存延续令牌用于下一次请求
            continuation_token = response.next_continuation_token;
        } else {
            // 如果没有更多页面，退出循环
            break;
        }
    }
    
    // 返回文件映射
    Ok(files)
}

/// 文件信息结构体
/// 
/// 用于存储文件的元数据信息，包括ETag
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 文件的ETag（用于比较文件是否相同）
    pub etag: String,
}

/// 同步操作枚举
/// 
/// 定义了两种同步操作类型：上传和删除
#[derive(Debug)]
pub enum SyncOperation {
    /// 上传操作
    /// 
    /// 将本地文件上传到S3
    Upload { 
        /// 本地文件路径（相对路径）
        local_path: String, 
        /// 远程S3键名
        remote_key: String 
    },
    /// 删除操作
    /// 
    /// 从S3删除文件
    Delete { 
        /// 远程S3键名
        remote_key: String 
    },
}

/// 生成同步操作队列
/// 
/// 比较本地和远程文件列表，生成需要执行的同步操作队列
/// 
/// # Arguments
/// 
/// * `local_files` - 本地文件信息映射
/// * `remote_files` - 远程文件信息映射
/// 
/// # Returns
/// 
/// * `Vec<SyncOperation>` - 同步操作队列
pub fn generate_sync_operations(
    local_files: &HashMap<String, FileInfo>,
    remote_files: &HashMap<String, FileInfo>,
) -> Vec<SyncOperation> {
    // 创建操作向量，用于存储同步操作
    let mut operations = Vec::new();
    
    // 遍历本地文件，确定需要上传的文件
    for (relative_path, local_info) in local_files {
        match remote_files.get(relative_path) {
            Some(remote_info) => {
                // 文件在远程存在，比较ETag
                if local_info.etag != remote_info.etag {
                    // ETag不同，需要上传
                    operations.push(SyncOperation::Upload {
                        local_path: relative_path.clone(),
                        remote_key: relative_path.clone(),
                    });
                }
                // ETag相同，跳过上传
            }
            None => {
                // 文件在远程不存在，需要上传
                operations.push(SyncOperation::Upload {
                    local_path: relative_path.clone(),
                    remote_key: relative_path.clone(),
                });
            }
        }
    }
    
    // 遍历远程文件，确定需要删除的文件
    for (relative_path, _) in remote_files {
        if !local_files.contains_key(relative_path) {
            // 文件在远程存在但在本地不存在，需要删除
            operations.push(SyncOperation::Delete {
                remote_key: relative_path.clone(),
            });
        }
    }
    
    // 返回操作队列
    operations
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
pub async fn execute_operations(
    client: &Client,
    local_dir: &str,
    bucket: &str,
    prefix: &str,
    operations: Vec<SyncOperation>,
) -> anyhow::Result<()> {
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

/// Push文件到S3的主要函数
/// 
/// 该函数负责整个同步过程：
/// 1. 加载环境变量
/// 2. 获取存储桶名称
/// 3. 处理前缀格式
/// 4. 获取S3客户端
/// 5. 扫描本地和远程文件
/// 6. 生成同步操作队列
/// 7. 执行操作队列
/// 
/// # Arguments
/// 
/// * `local_dir` - 本地目录路径
/// * `remote_dir` - 远程S3路径（前缀）
pub async fn push_files(local_dir: &str, remote_dir: &str) -> anyhow::Result<()> {
    // 加载环境变量（包括从.env文件）
    dotenvy::dotenv().ok();
    
    // 从环境变量获取存储桶
    let bucket = std::env::var("AWS_BUCKET")
        .map_err(|_| anyhow::anyhow!("AWS_BUCKET environment variable not set"))?;
    
    // 确保remote_dir以'/'结尾（如果不是空的话）
    let prefix = if !remote_dir.is_empty() && !remote_dir.ends_with('/') {
        format!("{}/", remote_dir)
    } else {
        remote_dir.to_string()
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
    let remote_files = get_s3_files(&client, &bucket, &prefix).await?;
    println!("Found {} remote files", remote_files.len());
    
    // 生成同步操作队列
    let operations = generate_sync_operations(&local_files, &remote_files);
    println!("Generated {} sync operations", operations.len());
    
    // 执行操作队列
    execute_operations(&client, local_dir, &bucket, &prefix, operations).await?;
    
    // 输出完成信息
    println!("Push completed successfully!");
    Ok(())
}