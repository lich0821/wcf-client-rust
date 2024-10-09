use async_trait::async_trait;
use log::info;

use crate::{handler::event_entity::{Event, EventHandler}, http_server::HttpServer, service::global_service::GLOBAL};

// 启动事件发布后，开启http服务
pub struct HttpServerHandler {
    pub id: String,
    pub http_server_running: bool,
    pub http_server: HttpServer,// 声明一个http server
}

#[async_trait]
impl EventHandler for HttpServerHandler {
    async fn handle(&mut self, event: Event) {
        
        if let Event::StartUp() = event {
            info!("HttpServer 启动");

            let global = GLOBAL.get().unwrap();
            let wechat_config = global.wechat_config.try_lock().unwrap();
            let port = wechat_config.http_server_port;

            let host_bytes = "0.0.0.0".to_string()
            .split('.')
            .map(|part| part.parse::<u8>().unwrap_or(0))
            .collect::<Vec<u8>>()
            .try_into()
            .map_err(|_| "Invalid host address".to_string()).unwrap();
            
            if !self.http_server_running {
                self.http_server.start(host_bytes, port).unwrap();
                self.http_server_running = true;
            }
            info!("服务启动，监听 http://{}:{}", "0.0.0.0", port);
            info!("浏览器访问 http://localhost:{}/swagger/ 查看文档", port);
        }
        
        if let Event::Shutdown() = event {
            info!("HttpServer 关闭");
            if self.http_server_running {
                match self.http_server.stop() {
                    Ok(()) => {
                        self.http_server_running = false;
                        ()
                    }
                    Err(e) => {
                        log::error!("http服务关闭失败 {}", e);
                    }
                }
            } else {
                info!("服务已停止");
            }
        }
    }
}
