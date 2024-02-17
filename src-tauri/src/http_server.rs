use log::{debug, error};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use crate::endpoints;
use crate::wcferry::WeChat;

pub struct HttpServer {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub wechat: Option<Arc<Mutex<WeChat>>>,
}

impl HttpServer {
    pub fn new() -> Self {
        HttpServer {
            shutdown_tx: None,
            wechat: None,
        }
    }

    pub fn start(&mut self, host: [u8; 4], port: u16, cburl: String) -> Result<(), String> {
        let wechat = Arc::new(Mutex::new(WeChat::new(true, cburl)));
        self.wechat = Some(wechat.clone());
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let addr = (host, port);

        let routes = endpoints::get_routes(wechat);
        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async {
            shutdown_rx.await.ok();
        });

        tokio::spawn(async move {
            server.await;
        });

        self.shutdown_tx = Some(shutdown_tx);
        debug!(
            "HTTP server started at http://{}:{}",
            host.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("."),
            port
        );

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            tokio::spawn(async move {
                if shutdown_tx.send(()).is_err() {
                    error!("Failed to send shutdown signal");
                }
            });
        }
        if let Some(wechat) = &self.wechat {
            let mut wechat = wechat.lock().unwrap(); // 获取 Mutex 的锁
            wechat.stop().unwrap();
        }
        debug!("HTTP server stopped");
        Ok(())
    }
}
