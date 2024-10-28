use std::sync::{Arc, Mutex};

use crate::wcferry::WeChat;

pub struct WechatService {
  pub wechat : Option<Arc<Mutex<WeChat>>>,
}

/** 单独处理 wechat */
impl WechatService {

  pub fn new(wechat : Option<Arc<Mutex<WeChat>>>) -> Self {
    WechatService { wechat }
  }
}