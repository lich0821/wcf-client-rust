use async_trait::async_trait;
use quickxml_to_serde::{xml_string_to_json, Config};

use crate::{handler::event_entity::{Event, EventHandler}, service::global_service::GLOBAL, wcferry::wcf};

/// 日志打印
pub struct EventMessageHandler {
    pub id: String,
}

#[async_trait]
impl EventHandler for EventMessageHandler {
    async fn handle(&mut self, event: Event) {
        if let Event::ClientMessage(ref msg) = event {
            if msg.r#type == 1 {
                
                log::debug!("[{}] 接收到事件推送：{:?}", self.id, msg);
                if !msg.content.contains("关键词")  {
                    return
                }

                // 解析xml 判断是否是at自己的信息
                let mut is_at_me = false;

                let global = GLOBAL.get().unwrap();
                let wechat_service = global.wechat_service.clone();
                let self_wx_id = wechat_service.lock().unwrap().get_self_wxid();

                let json = xml_string_to_json(msg.xml.clone(), &Config::new_with_defaults()).unwrap();
                let msgsource = json.get("msgsource");
                if msgsource.is_some(){
                    // 事件推送中包含 @某人，记录 @某人列表
                    let at_user_list = msgsource.unwrap().get("atuserlist");
                    if at_user_list.is_some() {
                        
                        let at_user_list = at_user_list.unwrap();
                        for at_user_item in at_user_list.as_str().unwrap().split(",") {
                            if self_wx_id == at_user_item {
                                is_at_me = true;
                            }
                        }
                    }
                }

                // 如果不是@我，则停止处理
                if !is_at_me {
                    return;
                }
                log::debug!("[{}] 接收到事件有人@我：{:?}", self.id, msg);
                let wechat_service = global.wechat_service.clone();
                let text_msg = wcf::TextMsg{
                    msg: format!("@{} {}",msg.sender.clone()," 事件推送有人at我".to_string()),
                    receiver: msg.roomid.clone(),
                    aters: msg.sender.clone()
                };
                log::debug!("发送的文本信息 -- {:?}",text_msg);
                wechat_service.lock().unwrap().send_text(text_msg);
            }
        }
    }
}



