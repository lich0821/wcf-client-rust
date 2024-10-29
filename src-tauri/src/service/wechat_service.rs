use std::sync::{Arc, Mutex};

use crate::wcferry::{wcf::RichText, WeChat};

pub struct WechatService {
  pub wechat : Option<Arc<Mutex<WeChat>>>,
}

/** 单独处理 wechat */
impl WechatService {

  pub fn new(wechat : Option<Arc<Mutex<WeChat>>>) -> Self {
    WechatService { wechat }
  }

  pub fn send_rich_text(&mut self, msg: RichText) {
    if let Some(wc) = &self.wechat {
      let wcc = wc.lock().unwrap();
      let _ = wcc.send_rich_text(msg);
    }
  }
}