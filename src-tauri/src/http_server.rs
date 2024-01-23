use crate::endpoints;
use log::{debug, error};
use tokio::sync::oneshot;

pub struct HttpServerState {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
}

impl HttpServerState {
    pub fn new() -> Self {
        HttpServerState { shutdown_tx: None }
    }
    pub fn start(&mut self, host: [u8; 4], port: u16) -> Result<(), String> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let addr = (host, port);

        // 使用 endpoints 模块中的函数获取所有路由
        let routes = endpoints::get_routes();

        let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(addr, async {
            shutdown_rx.await.ok();
        });

        tokio::spawn(async move {
            server.await;
        });

        self.shutdown_tx = Some(shutdown_tx);
        debug!(
            "HTTP server started at http://{}:{}",
            host.iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join("."),
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
        debug!("HTTP server stopped");
        Ok(())
    }
}
