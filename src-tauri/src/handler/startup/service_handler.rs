use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use log::info;

use crate::{handler::event_entity::{Event, EventHandler},  service::global_service::GLOBAL, wcferry::WeChat};

// 启动事件发布后，开启对应的所有服务
// wechat 客户端
// http_server 服务端
pub struct HttpServerHandler {
    pub id: String,
    pub http_server_running: bool,
}

#[async_trait]
impl EventHandler for HttpServerHandler {
    async fn handle(&mut self, event: Event) {
        
        if let Event::StartUp() = event {
            info!("HttpServer {} 启动", self.id);

            let global = GLOBAL.get().unwrap();

            // 初始化 wechat_client 服务
            let wechat_service_arc = global.wechat_service.clone();
            let mut wechat_service = wechat_service_arc.lock().unwrap();
            let wechat = Arc::new(Mutex::new(WeChat::new(true)));
            wechat_service.wechat = Some(wechat.clone());

            // 初始化 http_server 服务
            let wechat_config = global.wechat_config.read().unwrap();
            let port = wechat_config.http_server_port;
            let mut http_server_service = global.http_server_service.lock().unwrap();
            http_server_service.start(wechat.clone(), port).unwrap();
            info!("服务启动，监听 http://{}:{}", "0.0.0.0", port);
            info!("浏览器访问 http://localhost:{}/swagger/ 查看文档", port);

            // 初始化 socketio 服务
            let mut socket_service = global.socketio_service.lock().unwrap();
            socket_service.start(wechat_config.wsurl.clone())
        }
        
        if let Event::Shutdown() = event {
           
            let global = GLOBAL.get().unwrap();

            // 关闭 http_server 服务
            let mut http_server_service = global.http_server_service.lock().unwrap();
            match http_server_service.stop() {
                Ok(()) => {
                    self.http_server_running = false;
                    ()
                }
                Err(e) => {
                    log::error!("http服务关闭失败 {}", e);
                }
            }

            // 关闭 socketio 服务
            let mut socket_service = global.socketio_service.lock().unwrap();
            socket_service.stop();
        }
    }
}
