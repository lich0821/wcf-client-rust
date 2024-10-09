
use std::sync::{Arc, Mutex};

use tokio::{sync::broadcast, task};

use super::event_entity::{Event, EventHandler};

// 服务启动事件的监听
pub struct StartUpEventBus {
    pub broadcaster: Arc<Mutex<broadcast::Sender<Event>>>,
}

impl StartUpEventBus {
    pub fn new() -> Self {
      StartUpEventBus {
            broadcaster: Arc::new(Mutex::new(broadcast::channel(1000).0)),
        }
    }

    pub fn subscribe(&mut self,mut handler: Box<dyn EventHandler + Send + Sync>) {
      let broadcast: std::sync::MutexGuard<'_, broadcast::Sender<Event>> = self.broadcaster.lock().unwrap();
      let mut rx = broadcast.subscribe();
      task::spawn(async move {
        loop {
          match rx.recv().await {
              Ok(msg) => {
                handler.handle(msg).await;
              },
              Err(broadcast::error::RecvError::Closed) => break,
              Err(broadcast::error::RecvError::Lagged(msg)) => {
                println!("客户端丢失了消息: {:?}", msg);
                () 
              },
          }
        }
      });
    }

    pub fn send_message(&self, event: Event) {
        let broadcast = self.broadcaster.lock().unwrap();
        let _ = broadcast.send(event);
    }
}
