//! MIME类型处理工具
//!
//! 该模块提供了MIME类型相关的工具函数

use mime_guess::from_path;

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
pub fn get_content_type(file_path: &str) -> String {
    // 使用mime_guess库基于文件扩展名检测MIME类型
    from_path(file_path)
        .first_raw()
        .unwrap_or("binary/octet-stream")
        .to_string()
}
