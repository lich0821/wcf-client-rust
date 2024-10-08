use std::{fs, sync::{Arc, Mutex, OnceLock}};

use rand::Rng;

use crate::{service::message::{console_message_handler::ConsoleLogMessageHandler, http_message_handler::HttpMessageHandler, log_message_handler::LogMessageHandler}, wechat_config::WechatConfig};

use super::msg_event_mgr::MsgEventBus;


// 全局参数结构
pub struct GlobalState {
  pub wechat_config: Arc<Mutex<WechatConfig>>,
  pub msg_event_bus: Arc<Mutex<MsgEventBus>>,
}
// 全局变量
pub static GLOBAL: OnceLock<Arc<GlobalState>> = OnceLock::new();


// 初始化全局变量
pub fn initialize_global() {

   // 初始化配置信息
  let wechat_config: WechatConfig = init_config();

  // 消息总线
  let mut msg_event_bus = MsgEventBus::new();
  
  log::info!("-------------------微信监听初始化--------------------------------");
  let mut rng = rand::thread_rng();
  
  // 前台日志处理器
  let log_handler = Arc::new(LogMessageHandler {
      id: rng.gen::<u32>().to_string(),
  });
  msg_event_bus.subscribe(log_handler);

  // 控制台日志处理器
  // let console_log_handler = Arc::new(ConsoleLogMessageHandler {
  //   id: rng.gen::<u32>().to_string(),
  // });
  // msg_event_bus.subscribe(console_log_handler);

  // http 消息转发
  let http_handler = Arc::new(HttpMessageHandler {
    id: rng.gen::<u32>().to_string(),
  });
  msg_event_bus.subscribe(http_handler);
  
  let msg_event_bus_arc = Arc::new(Mutex::new(msg_event_bus));
  
  let global_state: GlobalState = GlobalState {
    wechat_config: Arc::new(Mutex::new(wechat_config)),
    msg_event_bus: msg_event_bus_arc,
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