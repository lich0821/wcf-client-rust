use std::sync::{Arc, Mutex, OnceLock};

use rand::Rng;

use crate::service::message::{console_message_handler::ConsoleLogMessageHandler, log_message_handler::LogMessageHandler};

use super::msg_event_mgr::MsgEventBus;


// 全局参数结构
pub struct GlobalState {
  pub msg_event_bus: Arc<Mutex<MsgEventBus>>,
}
// 全局变量
pub static GLOBAL: OnceLock<Arc<GlobalState>> = OnceLock::new();


// 初始化全局变量
pub fn initialize_global() {

  // 消息总线
  let mut msg_event_bus = MsgEventBus::new();
  
  log::info!("-------------------微信监听初始化--------------------------------");
  let mut rng = rand::thread_rng();
  
  // 前台日志处理器
  let log_handler = Arc::new(LogMessageHandler {
      id: rng.gen::<u32>().to_string(),
  });

  // 控制台日志处理器
  let console_log_handler = Arc::new(ConsoleLogMessageHandler {
    id: rng.gen::<u32>().to_string(),
  });

  msg_event_bus.subscribe(log_handler);
  msg_event_bus.subscribe(console_log_handler);   
  
  let msg_event_bus_arc = Arc::new(Mutex::new(msg_event_bus));
  
  let global_state: GlobalState = GlobalState {
    msg_event_bus: msg_event_bus_arc,
  };
  let _ = GLOBAL.set(Arc::new(global_state));
}
