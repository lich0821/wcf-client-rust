use std::sync::{Arc, Mutex};

use crate::wcferry::{wcf::{self, RichText}, WeChat};

pub struct WechatService {
  pub wechat : Option<Arc<Mutex<WeChat>>>,
}

/** 单独处理 wechat */
impl WechatService {

  pub fn new(wechat : Option<Arc<Mutex<WeChat>>>) -> Self {
    WechatService { wechat }
  }

  // 发送富文本信息
  pub fn send_rich_text(&mut self, msg: RichText) {
    if let Some(wc) = &self.wechat {
      let wcc = wc.lock().unwrap();
      let _ = wcc.send_rich_text(msg);
    }
  }

  // 获取自身的 wxid
  pub fn get_self_wxid(&mut self) ->  String  {
    if let Some(wc) = &self.wechat {
      let wcc = wc.lock().unwrap();
      return  wcc.get_self_wxid().unwrap_or("".to_string());
    }
    "".to_string()
  }

  // 发送文本信息
  pub fn send_text(&mut self, content: wcf::TextMsg) {
    if let Some(wc) = &self.wechat {
      let wcc = wc.lock().unwrap();
      let _ = wcc.send_text(content);
    }
  }
  
}