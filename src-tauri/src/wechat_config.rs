use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct WechatCoinfig {
    pub cburl: Vec<String>,
    pub wsurl: String,
    pub file_dir: String,
}
