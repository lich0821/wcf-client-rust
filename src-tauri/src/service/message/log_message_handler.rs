use async_trait::async_trait;

use super::event_entity::{Event, EventHandler};

/// 日志打印
pub struct LogMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for LogMessageHandler {
    async fn handle(&self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            log::info!("日志处理器 {} -- 接收到信息: {:?}", self.id, msg);
        }
    }
}



