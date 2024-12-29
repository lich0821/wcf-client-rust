use std::{fs, sync::{Arc, Mutex, OnceLock, RwLock}};

use rand::Rng;

use crate::{handler::{message::{event_message_handler::EventMessageHandler, http_message_handler::HttpMessageHandler, log_message_handler::LogMessageHandler, socketio_message_handler::SocketIOMessageHandler}, msg_event_mgr::MsgEventBus, startup::service_handler::HttpServerHandler, startup_event_mgr::StartUpEventBus}, service::http_server_service::HttpServerService, wechat_config::WechatConfig};

use super::{socketio_service::SocketIOService, wechat_service::WechatService};


// 全局参数结构
pub struct GlobalState {
  pub wechat_config: RwLock<WechatConfig>,
  pub msg_event_bus: Arc<Mutex<MsgEventBus>>,
  pub startup_event_bus: Arc<Mutex<StartUpEventBus>>,
  pub wechat_service: Arc<Mutex<WechatService>>,
  pub http_server_service: Arc<Mutex<HttpServerService>>,
  pub socketio_service: Arc<Mutex<SocketIOService>>
}
// 全局变量
pub static GLOBAL: OnceLock<Arc<GlobalState>> = OnceLock::new();


// 初始化全局变量
pub fn initialize_global() {

   // 初始化配置信息
  let wechat_config: WechatConfig = init_config();

  let mut rng = rand::thread_rng();

  log::info!("-------------------服务启动监听初始化 开始--------------------------------");
  // 服务启动总线
  let mut startup_event_bus = StartUpEventBus::new();

  let http_server_handler = Box::new(
    HttpServerHandler{
      id: rng.gen::<u32>().to_string(),
      http_server_running: false,
    }
  );
  startup_event_bus.subscribe(http_server_handler);

  log::info!("-------------------服务启动监听初始化 结束--------------------------------");


  log::info!("-------------------微信消息监听初始化 开始--------------------------------");
  // 消息总线
  let mut msg_event_bus = MsgEventBus::new();
  
  // 前台日志处理器
  let log_handler = Box::new(LogMessageHandler {
      id: rng.gen::<u32>().to_string(),
  });
  msg_event_bus.subscribe(log_handler);

  // 控制台日志处理器
  // let console_log_handler = Arc::new(ConsoleLogMessageHandler {
  //   id: rng.gen::<u32>().to_string(),
  // });
  // msg_event_bus.subscribe(console_log_handler);

  // http 消息转发
  let http_handler = Box::new(HttpMessageHandler {
    id: rng.gen::<u32>().to_string(),
  });
  msg_event_bus.subscribe(http_handler);


  // 事件处理
  let event_msg_handler = Box::new(EventMessageHandler {
    id: "@事件处理器".to_string(),
  });
  msg_event_bus.subscribe(event_msg_handler);

  // socketIO 消息转发
  let socket_io_handler = Box::new(SocketIOMessageHandler {
    id: rng.gen::<u32>().to_string(),
  });
  msg_event_bus.subscribe(socket_io_handler);


  log::info!("-------------------微信消息监听初始化 结束--------------------------------");

  let global_state: GlobalState = GlobalState {
    wechat_config: RwLock::new(wechat_config),
    msg_event_bus: Arc::new(Mutex::new(msg_event_bus)),
    startup_event_bus: Arc::new(Mutex::new(startup_event_bus)),
    wechat_service: Arc::new(Mutex::new(WechatService::new(None))),
    http_server_service:  Arc::new(Mutex::new(HttpServerService::new())),
    socketio_service: Arc::new(Mutex::new(SocketIOService::new()))
  };
  let _ = GLOBAL.set(Arc::new(global_state));
}


// 读取配置
fn init_config() -> WechatConfig {
  // 获取应用安装目录的路径
  // let install_dir = resolve_path(&app, ".", None).map_err(|e| e.to_string())?;
  // 定义文件路径
  let file_path = ".\\config.json5";

  // 尝试创建并写入文件
  let file_str = fs::read_to_string(&file_path).unwrap();

  let wechat_config: WechatConfig = serde_json::from_str(&file_str).unwrap();
  wechat_config
}