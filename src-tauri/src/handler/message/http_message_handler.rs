use async_trait::async_trait;

use crate::{handler::event_entity::{Event, EventHandler}, service::global_service::GLOBAL};

use serde_json::json;

/// 配置 http 回调地址后，将调用设置的url，
pub struct HttpMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for HttpMessageHandler {
    async fn handle(&mut self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            let global = GLOBAL.get().unwrap();
            let k_config = global.wechat_config.try_lock().unwrap();
            let cburl = k_config.cburl.clone();
            if cburl.is_empty() {
                return;
            }

             for url in cburl {
                log::debug!("http服务 {} 回调地址为: {:?}", self.id, url.clone());
                if !url.starts_with("http") {
                    log::error!("http 转发消息失败，回调地址不合法");
                    continue;
                }

                let res = ureq::post(&url).send_json(json!(&msg));
                match res {
                    Ok(rsp) => {
                        if rsp.status() != 200 {
                            log::error!("转发消息失败，状态码: {}", rsp.status());
                        }
                        log::debug!("{}", rsp.into_string().unwrap());
                    }
                    Err(e) => {
                        log::error!("转发消息失败：{}", e);
                    }
                } 
            }
        }
    }
}
