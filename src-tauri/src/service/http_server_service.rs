use log::{debug, error, info};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use crate::endpoints;
use crate::wcferry::WeChat;

pub struct HttpServerService {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub wechat: Option<Arc<Mutex<WeChat>>>,
}

impl HttpServerService {
    pub fn new() -> Self {
        HttpServerService {
            shutdown_tx: None,
            wechat: None,
        }
    }

    pub fn start(&mut self, wechat: Arc<Mutex<WeChat>>, port: u16) -> Result<(), String> {
        info!("HttpServerService 启动");

        let host = [0,0,0,0];

        self.wechat = Some(wechat.clone());
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let addr: ([u8;4], u16) = (host, port);

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
        info!("HttpServerService 关闭");
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