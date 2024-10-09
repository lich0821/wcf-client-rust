use async_trait::async_trait;

use crate::{handler::event_entity::{Event, EventHandler}, service::global_service::GLOBAL};

/// 日志打印
pub struct LogMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for LogMessageHandler {
    async fn handle(&mut self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            let global = GLOBAL.get().unwrap();
            let wechat_config = global.wechat_config.read().unwrap();
            let show = wechat_config.front_msg_show.clone();
            if show {
                log::info!("日志处理器 {} -- 接收到信息: {:?}", self.id, msg);
            }
        }
    }
}



