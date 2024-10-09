use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WechatConfig {
    // http 回调地址
    pub cburl: Vec<String>,
    // http 回调地址
    pub http_server_port: u16,
    // websocket 地址
    pub wsurl: String,
    // 本地文件存储路径
    pub file_dir: String,
}
