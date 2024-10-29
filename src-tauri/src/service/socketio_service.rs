
use std::{sync::Arc, time::Duration};

use log::{debug, info};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::{service::global_service::GLOBAL, wcferry::wcf};

use futures_util::FutureExt;
use rust_socketio::{
    asynchronous::{Client, ClientBuilder},
    Payload,
};
/** websocket 服务 */
#[derive(Clone)]
pub struct SocketIOService {
    pub socketio_url: String,
    pub socketio_client: Arc<Mutex<Option<Client>>>,
}

impl SocketIOService {

    // 初始化服务
    pub fn new() -> Self {
        SocketIOService{
            socketio_url: "".to_string(),
            socketio_client: Arc::new(Mutex::new(None)),
        }
    }

    // 启动服务
    pub fn start (&mut self, wsurl: String) {
        if !wsurl.is_empty() {
            self.socketio_url = wsurl;
            let mut cloned_self = self.clone();
            tokio::spawn(async move {
                let _ = cloned_self.connect().await;
            });
        }
    }
  
    pub fn stop (&mut self) {
        let _ = self.disconnect();
    }


    // 发起服务器端连接
    async fn connect(&mut self) -> std::io::Result<()> {
                
        let callback = |payload: Payload, socket: Client| {
            async move {
                match payload {
                    Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
                    Payload::Text(_) => todo!(),
                    _ => (),
                }
                socket
                    .emit("test", json!({"got ack": true}))
                    .await
                    .expect("Server unreachable");
            }
            .boxed()
        };

        // 从服务端接收到消息
        let func_send_rich_txt = |payload: Payload, _| {
            async  move{
                match payload {
                    Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
                    Payload::Text(res) => {
                        log::info!("---- {:?}", res);
                        let json = res[0].as_array().unwrap();
                        let rich_msg_vec: Result<Vec<wcf::RichText>, serde_json::Error> = json
                            .into_iter()
                            .map(|value| {
                                return serde_json::from_value(value.clone());
                            })
                            .collect();
                        let global = GLOBAL.get().unwrap();
                        for rich_text in rich_msg_vec.unwrap() {
                            let wechat_arc = global.wechat_service.clone();
                            let mut wechat1 = wechat_arc.lock().unwrap();
                            wechat1.send_rich_text(rich_text);
                        }
                    }
                    _ => (),
                }
            }.boxed()
        };

        // 发起连接
        let socket_client = ClientBuilder::new(self.socketio_url.clone())
            .namespace("/")
            .on("MSG", callback)
            .on("error", |err, _| {
                async move { eprintln!("Error: {:#?}", err) }.boxed()
            })
            .on("PONG", |payload, _| {
                async move { debug!("socketio pong info: {:#?}", payload) }.boxed()
            })
            .on("FuncSendRichTxt", func_send_rich_txt)
            .connect()
            .await             
            .expect("Connection failed")
            ;
         
        let mut temp = self.socketio_client.lock().await;
        *temp = Some(socket_client);

        let task_ping = self.socketio_client.clone();
        tokio::spawn(async move  {
            loop {
               
                if let Some(ref client) = *task_ping.lock().await {

                    let json_payload = json!({"type": "ping"});
                    client
                    .emit("PING", json_payload)
                    .await
                    .expect("Server unreachable");
                }
                // 睡眠定期推送数据
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        });
        log::info!("开启websocket {:?}", self.socketio_url.clone());
        Ok(())
    }

    // 断开连接
    fn disconnect(&mut self) -> std::io::Result<()> {
        let t_socketio_client = self.socketio_client.clone();
        tokio::spawn(async move {
            if let Some(ref client) = *t_socketio_client.lock().await {
                let _ = client.disconnect().await.expect("Disconnect failed");
                info!("socketIo 已断开连接");
            }
        });
        self.socketio_client = Arc::new(Mutex::new(None));
        Ok(())
    }

    // 发送消息到服务器端
    pub fn send_msg_to_server(&mut self, payload: Value) {
        let task_msg = self.socketio_client.clone();
        tokio::spawn(async move {
            if let Some(ref client) = *task_msg.lock().await {
                client.emit("MSG", payload).await.expect("Server unreachable");
            }
        });
    }
}