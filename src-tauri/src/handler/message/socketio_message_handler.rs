use async_trait::async_trait;
use serde_json::json;
use crate::{handler::event_entity::{Event, EventHandler}, service::global_service::GLOBAL};

// 控制台日志打印
pub struct SocketIOMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for SocketIOMessageHandler {
    async fn handle(&mut self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            let global = GLOBAL.get().unwrap();
            let socket_arc = global.socketio_service.clone();
            let mut client  = socket_arc.lock().unwrap();
            client.send_msg_to_server(json!(msg));
        }
    }
}



