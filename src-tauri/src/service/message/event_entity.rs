use async_trait::async_trait;

use crate::wcferry::wcf;

#[derive(Clone)]
pub enum Event {
    ClientMessage(wcf::WxMsg),
}

#[async_trait]
pub trait EventHandler {
    async fn handle(&self, event: Event);
}