use crate::wcferry::WeChat;
use log::{debug, error};
use serde::Serialize;
use tokio::sync::oneshot;
use warp::Filter;
use warp::Rejection;
use warp::Reply;

pub struct HttpServer {
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    pub wechat: Option<WeChat>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    status: u16,           // 状态码，可以根据需要定义
    error: Option<String>, // 消息
    data: T,               // 数据字段，泛型类型，可以根据需要定义
}

impl HttpServer {
    pub fn new() -> Self {
        HttpServer {
            shutdown_tx: None,
            wechat: None,
        }
    }
    pub fn start(&mut self, host: [u8; 4], port: u16) -> Result<(), String> {
        self.wechat = Some(WeChat::new(true));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let addr = (host, port);

        let routes = self.get_routes();
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
        self.wechat.as_mut().unwrap().stop().unwrap();
        Ok(())
    }

    fn get_routes(&self) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let gets = warp::path("api")
            .and(warp::path("v1"))
            .and(self.is_login().or(self.get_self_wxid()));

        let posts = warp::post()
            .and(warp::path("api"))
            .and(warp::path("v1"))
            .and(self.get_self_wxid());

        gets.or(posts)
    }

    fn is_login(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let status = self.wechat.clone().unwrap().is_login().unwrap();
        warp::path("is_login").map(move || {
            let response = ApiResponse {
                status: 0,
                error: None,
                data: status,
            };

            warp::reply::json(&response)
        })
    }

    fn get_self_wxid(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        let wxid = self
            .wechat
            .clone()
            .unwrap()
            .get_self_wxid()
            .unwrap()
            .unwrap();
        warp::path("self_wxid").map(move || {
            let response = ApiResponse {
                status: 0,
                error: None,
                data: wxid.clone(),
            };

            warp::reply::json(&response)
        })
    }
}
