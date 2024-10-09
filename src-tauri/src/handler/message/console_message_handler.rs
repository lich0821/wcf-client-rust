use async_trait::async_trait;

use crate::handler::event_entity::{Event, EventHandler};

// 控制台日志打印
pub struct ConsoleLogMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for ConsoleLogMessageHandler {
    async fn handle(&mut self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            println!("控制台日志处理器 {} -- 接收到信息: {:?}", self.id, msg);
        }
    }
}



