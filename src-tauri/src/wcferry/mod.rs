use libloading::{Library, Symbol};
use log::{debug, error, info, warn};
use nng::options::{Options, RecvTimeout, SendTimeout};
use prost::Message;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};
use std::{
    env,
    thread::{self, sleep},
    time::Duration,
};

const CMD_URL: &'static str = "tcp://127.0.0.1:10086";
const MSG_URL: &'static str = "tcp://127.0.0.1:10087";

pub mod wcf {
    include!("wcf.rs");
}

pub mod roomdata {
    include!("roomdata.rs");
}

use wcf::{request::Msg as ReqMsg, response::Msg as RspMsg, Functions, WxMsg};

use crate::{handler::event_entity::Event, service::global_service::GLOBAL};

#[macro_export]
macro_rules! create_request {
    ($func:expr) => {
        wcf::Request {
            func: $func.into(),
            msg: None,
        }
    };
    ($func:expr, $msg:expr) => {
        wcf::Request {
            func: $func.into(),
            msg: Some($msg),
        }
    };
}

#[macro_export]
macro_rules! try_cmd {
    ($expr:expr, $err_msg:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => {
                error!("{}: {:?}", $err_msg, e);
                return Err($err_msg.into());
            }
        }
    };
}

#[macro_export]
macro_rules! process_response {
    ($rsp:expr, Status $expected:expr, $err_msg:expr) => {
        match $rsp {
            Some(RspMsg::Status(status)) => Ok(status == $expected),
            _ => {
                log::error!("{}", $err_msg);
                Err($err_msg.into())
            }
        }
    };
    ($rsp:expr, $msg_variant:ident, $err_msg:expr) => {
        match $rsp {
            Some(RspMsg::$msg_variant(data)) => Ok(data),
            _ => {
                log::error!("{}", $err_msg);
                Err($err_msg.into())
            }
        }
    };
}

#[macro_export]
macro_rules! execute_wcf_command {
    ($self:ident, $func:expr, $msg_variant:ident, $desc:expr) => {{
        let req = create_request!($func);
        let rsp = try_cmd!($self.send_cmd(req), $desc.to_owned() + "命令发送失败");
        process_response!(rsp, $msg_variant, $desc.to_owned() + "失败")
    }};
    ($self:ident, $func:expr, $msg:expr, $msg_variant:ident, $desc:expr) => {{
        let req = create_request!($func, $msg);
        let rsp = try_cmd!($self.send_cmd(req), $desc.to_owned() + "命令发送失败");
        process_response!(rsp, $msg_variant, $desc.to_owned() + "失败")
    }};
    ($self:ident, $func:expr, Status $expected_val:expr, $desc:expr) => {{
        let req = create_request!($func);
        let rsp = try_cmd!($self.send_cmd(req), $desc.to_owned() + "命令发送失败");
        process_response!(rsp, Status $expected_val, $desc.to_owned() + "失败")
    }};
    ($self:ident, $func:expr, $msg:expr, Status $expected_val:expr, $desc:expr) => {{
        let req = create_request!($func, $msg);
        let rsp = try_cmd!($self.send_cmd(req), $desc.to_owned() + "命令发送失败");
        process_response!(rsp, Status $expected_val, $desc.to_owned() + "失败")
    }};
}

pub struct RoomMember {
    /// 微信ID
    pub wxid: String,
    /// 群内昵称
    pub name: String,
    pub state: i32,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SelfInfo {
    /// 微信ID
    pub wxid: String,
    /// 昵称
    pub name: String,
    /// 手机号
    pub mobile: String,
    /// 文件/图片等父路径
    pub home: String,
    /// 小头像
    pub small_head_url: Option<String>,
    /// 大头像
    pub big_head_url: Option<String>,
    /// 别名
    pub alias: String,
}

#[derive(Debug)]
pub struct WeChat {
    pub dll: Arc<Library>,
    pub listening: Arc<AtomicBool>,
    pub cmd_socket: nng::Socket,
    pub msg_socket: Option<nng::Socket>,
}

impl Clone for WeChat {
    fn clone(&self) -> Self {
        WeChat {
            dll: Arc::clone(&self.dll),
            listening: Arc::clone(&self.listening),
            cmd_socket: self.cmd_socket.clone(),
            msg_socket: self.msg_socket.clone(),
        }
    }
}

impl Default for WeChat {
    fn default() -> Self {
        WeChat::new(false)
    }
}

impl WeChat {
    pub fn new(debug: bool) -> Self {
        let dll_path = env::current_dir()
            .unwrap()
            .join("src\\wcferry\\lib\\sdk.dll");
        let dll = unsafe { Library::new(dll_path).unwrap() };
        let _ = WeChat::start(&dll, debug);
        let cmd_socket = WeChat::connect(&CMD_URL).unwrap();
        let mut wc = WeChat {
            dll: Arc::new(dll),
            listening: Arc::new(AtomicBool::new(false)),
            cmd_socket,
            msg_socket: None,
        };
        info!("注入成功");
        /* while !wc.clone().is_login().unwrap() {
            sleep(Duration::from_secs(1));
        } */
        if(wc.clone().is_login().unwrap()){
            info!("微信登录成功");  
        }else{
            info!("微信未登录");
        }
        let mut wc_clone = wc.clone(); // 克隆一份 wc 给新线程使用
        thread::spawn(move || { // 启动一个新线程
            info!("启动登录状态检测线程..."); // Starting login status check thread...
            loop {
                match wc_clone.is_login() {
                    Ok(true) => {
                        // 已经登录
                        info!("检测到微信已登录，尝试启用消息接收..."); // WeChat logged in detected, attempting to enable message receiving...
                        // 检查是否已经启用了监听，避免重复启用 (可选，取决于 enable_recv_msg 的实现)
                        if !wc_clone.listening.load(Ordering::SeqCst) {
                            match wc_clone.enable_recv_msg() {
                                Ok(_) => {
                                    // enable_recv_msg 成功后，应该在内部设置 listening = true
                                    // 如果 enable_recv_msg 不设置，需要在这里手动设置:
                                    // wc_clone.listening.store(true, Ordering::SeqCst);
                                    info!("消息接收已成功启用。"); // Message receiving enabled successfully.
                                    break; // 成功启用后退出循环
                                }
                                Err(e) => {
                                    // error!("启用消息接收失败: {:?}，将在一秒后重试...", e); // Failed to enable message receiving, will retry in 1 second...
                                    // 这里可以选择继续尝试或在某些错误下退出
                                    thread::sleep(Duration::from_secs(1));
                                }
                            }
                        } else {
                            info!("消息接收已启用，检测线程退出。"); // Message receiving already enabled, check thread exiting.
                            break; // 如果已经启用了，也退出循环
                        }
                    }
                    Ok(false) => {
                        // 尚未登录
                        //info!("微信未登录，1秒后重新检测..."); // WeChat not logged in, checking again in 1 second...
                        thread::sleep(Duration::from_secs(1)); // 等待1秒
                    }
                    Err(e) => {
                        // is_login 调用出错
                        error!("检测登录状态时发生错误: {:?}，检测进程退出", e); // Error checking login status, retrying in 1 second...
                        break;
                    }
                }
            }
            info!("登录状态检测线程结束。"); // Login status check thread finished.
        });
        wc
    }

    fn start(dll: &Library, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
        type WxInitSDK = unsafe extern "C" fn(bool, i32) -> i32;
        let wx_init_sdk: Symbol<WxInitSDK> = unsafe { dll.get(b"WxInitSDK")? };

        let port = 10086;
        let result = unsafe { wx_init_sdk(debug, port) };
        if result != 0 {
            return Err("WxInitSDK 启动失败".into());
        }
        Ok(())
    }

    fn connect(url: &str) -> Result<nng::Socket, Box<dyn std::error::Error>> {
        let client = try_cmd!(nng::Socket::new(nng::Protocol::Pair1), "Socket 创建失败");
        try_cmd!(
            client.set_opt::<RecvTimeout>(Some(Duration::from_millis(5000))),
            "接收超时设置失败"
        );
        try_cmd!(
            client.set_opt::<SendTimeout>(Some(Duration::from_millis(5000))),
            "发送超时设置失败"
        );
        try_cmd!(client.dial(url), "连接服务失败");
        Ok(client)
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.listening.load(Ordering::Relaxed) {
            let _ = self.disable_recv_msg();
            self.listening.store(false, Ordering::Relaxed);
        }
        self.cmd_socket.close();

        type WxDestroySDK = unsafe extern "C" fn() -> i32;
        let wx_destroy_sdk: Symbol<WxDestroySDK> = unsafe { self.dll.get(b"WxDestroySDK")? };

        let result = unsafe { wx_destroy_sdk() };
        if result != 0 {
            return Err("WxDestroySDK 停止失败".into());
        }
        debug!("服务已停止: {}", CMD_URL);
        Ok(())
    }

    fn send_cmd(&self, req: wcf::Request) -> Result<Option<RspMsg>, Box<dyn std::error::Error>> {
        let mut buf = Vec::with_capacity(req.encoded_len());
        try_cmd!(req.encode(&mut buf), "编码失败");
        let msg = nng::Message::from(&buf[..]);
        try_cmd!(self.cmd_socket.send(msg), "消息发送失败");
        let mut msg = try_cmd!(self.cmd_socket.recv(), "消息接收失败");
        let rsp = try_cmd!(wcf::Response::decode(msg.as_slice()), "解码失败");
        msg.clear();
        Ok(rsp.msg)
    }

    pub fn refresh_qrcode(&self) -> Result<String, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncRefreshQrcode, Str, "获取登录二维码 ")
    }

    pub fn is_login(&self) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncIsLogin, Status 1, "获取登录状态")
    }

    pub fn get_self_wxid(&self) -> Result<String, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncGetSelfWxid, Str, "获取 wxid ")
    }

    pub fn get_user_info(&self) -> Result<SelfInfo, Box<dyn std::error::Error>> {
        let user_info_result: Result<wcf::UserInfo, Box<dyn std::error::Error>> =
            execute_wcf_command!(self, Functions::FuncGetUserInfo, Ui, "获取用户信息");
        let user_info = user_info_result?;
        let mut self_info = SelfInfo {
            wxid: user_info.wxid.clone(),
            name: user_info.name,
            mobile: user_info.mobile,
            home: user_info.home,
            small_head_url: None,
            big_head_url: None,
            alias: user_info.alias,
        };
        let query = wcf::DbQuery {
            db: String::from("MicroMsg.db"),
            sql: String::from(format!(
                "select * from ContactHeadImgUrl where usrName = '{}'",
                user_info.wxid
            )),
        };
        let rows_result: Result<wcf::DbRows, Box<dyn std::error::Error>> = execute_wcf_command!(
            self,
            Functions::FuncExecDbQuery,
            ReqMsg::Query(query),
            Rows,
            "查询用户头像信息"
        );
        let rows = rows_result?.rows;
        if rows.len() > 0 {
            let row = rows.get(0).expect("头像索引获取失败");
            let fields = &row.fields;
            for field in fields.into_iter() {
                if field.column.eq("smallHeadImgUrl") {
                    self_info.small_head_url = Some(
                        String::from_utf8(field.content.clone())
                            .map_err(|e| format!("微信小头像解析失败: {}", e.to_string()))?,
                    );
                } else if field.column.eq("bigHeadImgUrl") {
                    self_info.big_head_url = Some(
                        String::from_utf8(field.content.clone())
                            .map_err(|e| format!("微信大头像解析失败: {}", e.to_string()))?,
                    );
                }
            }
        }

        Ok(self_info)
    }

    pub fn get_contacts(&self) -> Result<wcf::RpcContacts, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncGetContacts, Contacts, "获取联系人列表")
    }

    pub fn get_dbs(&self) -> Result<wcf::DbNames, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncGetDbNames, Dbs, "获取数据库名称")
    }

    pub fn get_tables(&self, db: String) -> Result<wcf::DbTables, Box<dyn std::error::Error>> {
        execute_wcf_command!(
            self,
            Functions::FuncGetDbTables,
            ReqMsg::Str(db),
            Tables,
            "获取数据表"
        )
    }

    pub fn enable_recv_msg(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        fn listening_msg(wechat: &mut WeChat, tx: SyncSender<wcf::WxMsg>) {
            while wechat.listening.load(Ordering::Relaxed) {
                match wechat.msg_socket.as_ref().unwrap().recv() {
                    Ok(buf) => {
                        let rsp = match wcf::Response::decode(buf.as_slice()) {
                            Ok(rsp) => rsp,
                            Err(e) => {
                                warn!("消息解码失败: {}", e);
                                break;
                            }
                        };
                        if let Some(RspMsg::Wxmsg(msg)) = rsp.msg {
                            match tx.send(msg) {
                                Ok(_) => {
                                    debug!("消息入队成功");
                                }
                                Err(e) => {
                                    error!("消息入队失败: {}", e);
                                }
                            }
                        } else {
                            warn!("获取消息失败: {:?}", rsp.msg);
                            break;
                        }
                    }
                    Err(nng::Error::TimedOut) => {
                        debug!("消息接收超时，继续等待...");
                        continue;
                    }
                    Err(e) => {
                        warn!("消息接收失败: {}", e);
                        break;
                    }
                }
            }
            let _ = wechat.disable_recv_msg().unwrap();
        }

        fn forward_msg(wechat: &mut WeChat, rx: Receiver<WxMsg>) {
            while wechat.listening.load(Ordering::Relaxed) {
                match rx.recv() {
                    Ok(msg) => {
                        // 发送到消息监听器中
                        let global = GLOBAL.get().unwrap();
                        let event_bus = global.msg_event_bus.lock().unwrap();
                        let _ = event_bus.send_message(Event::ClientMessage(msg.clone()));
                    }
                    Err(e) => {
                        error!("消息出队失败: {}", e);
                        break;
                    }
                }
            }
        }

        if self.listening.load(Ordering::Relaxed) {
            warn!("已经启用消息接收");
            return Ok(true);
        }

        let req = create_request!(Functions::FuncEnableRecvTxt, wcf::request::Msg::Flag(true));
        let rsp = try_cmd!(self.send_cmd(req), "启用消息接收命令发送失败");

        match rsp.unwrap() {
            RspMsg::Status(status) => {
                if status == 0 {
                    let (tx, rx) = mpsc::sync_channel::<wcf::WxMsg>(100);
                    self.msg_socket = Some(WeChat::connect(MSG_URL).unwrap());
                    self.listening.store(true, Ordering::Relaxed);
                    let mut wc1 = self.clone();
                    let mut wc2 = self.clone();
                    thread::spawn(move || listening_msg(&mut wc1, tx));
                    thread::spawn(move || forward_msg(&mut wc2, rx));
                    return Ok(true);
                } else {
                    error!("启用消息接收失败：{}", status);
                    return Ok(false);
                }
            }
            _ => {
                return Err("启用消息接收失败".into());
            }
        };
    }

    pub fn disable_recv_msg(&mut self) -> Result<i32, Box<dyn std::error::Error>> {
        if !self.listening.load(Ordering::Relaxed) {
            return Ok(0);
        }

        let req = create_request!(Functions::FuncDisableRecvTxt);
        let rsp = try_cmd!(self.send_cmd(req), "停止消息接收命令发送失败");

        match rsp.unwrap() {
            RspMsg::Status(status) => {
                // TODO: 处理状态码
                self.msg_socket.take().map(|s| s.close());
                self.listening.store(false, Ordering::Relaxed);
                return Ok(status);
            }
            _ => {
                return Err("停止消息接收失败".into());
            }
        };
    }

    pub fn get_msg_types(&self) -> Result<wcf::MsgTypes, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncGetMsgTypes, Types, "获取消息类型")
    }

    pub fn refresh_pyq(&self, id: u64) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncRefreshPyq, ReqMsg::Ui64(id), Status 0, "刷新朋友圈")
    }

    pub fn send_text(&self, text: wcf::TextMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncSendTxt, ReqMsg::Txt(text), Status 0, "发送文本消息")
    }

    pub fn send_image(&self, img: wcf::PathMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncSendImg, ReqMsg::File(img), Status 0, "发送图片消息")
    }

    pub fn send_file(&self, file: wcf::PathMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncSendFile, ReqMsg::File(file), Status 0, "发送文件消息")
    }

    pub fn send_rich_text(&self, msg: wcf::RichText) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncSendRichTxt, ReqMsg::Rt(msg), Status 0, "发送卡片消息")
    }

    pub fn send_pat_msg(&self, msg: wcf::PatMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncSendPatMsg, ReqMsg::Pm(msg), Status 1, "发送拍一拍消息")
    }

    pub fn forward_msg(&self, msg: wcf::ForwardMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncForwardMsg, ReqMsg::Fm(msg), Status 1, "转发消息")
    }

    pub fn save_audio(&self, am: wcf::AudioMsg) -> Result<String, Box<dyn std::error::Error>> {
        execute_wcf_command!(
            self,
            Functions::FuncGetAudioMsg,
            ReqMsg::Am(am),
            Str,
            "保存语音"
        )
    }

    pub fn decrypt_image(&self, msg: wcf::DecPath) -> Result<String, Box<dyn std::error::Error>> {
        execute_wcf_command!(
            self,
            Functions::FuncDecryptImage,
            ReqMsg::Dec(msg),
            Str,
            "解密图片"
        )
    }

    pub fn download_attach(&self, msg: wcf::AttachMsg) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncDownloadAttach, ReqMsg::Att(msg), Status 0, "下载附件")
    }

    pub fn recv_transfer(&self, msg: wcf::Transfer) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncRecvTransfer, ReqMsg::Tf(msg), Status 1, "接收转账")
    }

    pub fn query_sql(&self, msg: wcf::DbQuery) -> Result<wcf::DbRows, Box<dyn std::error::Error>> {
        execute_wcf_command!(
            self,
            Functions::FuncExecDbQuery,
            ReqMsg::Query(msg),
            Rows,
            "查询 SQL "
        )
    }

    pub fn accept_new_friend(
        &self,
        msg: wcf::Verification,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncAcceptFriend, ReqMsg::V(msg), Status 1, "通过好友申请")
    }

    pub fn add_chatroom_member(
        &self,
        msg: wcf::MemberMgmt,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncAddRoomMembers, ReqMsg::M(msg), Status 1, "添加群成员")
    }

    pub fn invite_chatroom_member(
        &self,
        msg: wcf::MemberMgmt,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncInvRoomMembers, ReqMsg::M(msg), Status 1, "邀请群成员")
    }

    pub fn delete_chatroom_member(
        &self,
        msg: wcf::MemberMgmt,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncDelRoomMembers, ReqMsg::M(msg), Status 1, "删除群成员")
    }

    pub fn revoke_msg(&self, id: u64) -> Result<bool, Box<dyn std::error::Error>> {
        execute_wcf_command!(self, Functions::FuncRevokeMsg, ReqMsg::Ui64(id), Status 1, "撤回消息")
    }

    pub fn query_room_member(
        &self,
        room_id: String,
    ) -> Result<Option<Vec<RoomMember>>, Box<dyn std::error::Error>> {
        let query = wcf::DbQuery {
            db: String::from("MicroMsg.db"),
            sql: String::from(format!(
                "select * from ChatRoom where ChatRoomName = '{}'",
                room_id.clone()
            )),
        };
        let rows: Result<wcf::DbRows, Box<dyn std::error::Error>> = execute_wcf_command!(
            self,
            Functions::FuncExecDbQuery,
            ReqMsg::Query(query),
            Rows,
            "查询群成员"
        );
        let db_rows = rows?.rows;
        if db_rows.len() == 0 {
            return Ok(None);
        }
        let room_row = db_rows.get(0).expect("获取群聊索引失败");
        let fields = &room_row.fields;
        for field in fields.into_iter() {
            if field.column.eq("RoomData") {
                debug!("roomdata field content:{:?}", field.content);
                let room_data = roomdata::RoomData::decode(field.content.as_slice())?;
                debug!("群聊：{} 总计：{}人", room_id, room_data.members.len());
    
                // 构建需要查询昵称的wxid列表
                let wxids: Vec<_> = room_data.members
                    .iter()
                    .filter(|m| m.name.as_deref().unwrap_or_default().is_empty())
                    .map(|m| m.wxid.clone())
                    .collect();
    
                // 构建昵称映射表
                let nick_name_map = if !wxids.is_empty() {
                    let params = wxids.iter()
                        .map(|id| format!("'{}'", id.replace("'", "''")))
                        .collect::<Vec<_>>()
                        .join(",");
                    
                    let query = wcf::DbQuery {
                        db: String::from("MicroMsg.db"),
                        sql: format!(
                            "SELECT UserName, NickName FROM Contact WHERE UserName IN ({})",
                            params
                        ),
                    };
                    let rows: Result<wcf::DbRows, Box<dyn std::error::Error>> = execute_wcf_command!(
                        self,
                        Functions::FuncExecDbQuery,
                        ReqMsg::Query(query),
                        Rows,
                        "查询昵称"
                    );
                    let db_rows = rows?.rows;
                    db_rows
                    .into_iter()
                    .filter_map(|row| {
                        let username = row.fields.iter()
                            .find(|f| f.column == "UserName")
                            .and_then(|f| String::from_utf8_lossy(&f.content).parse().ok());
                        
                        let nickname = row.fields.iter()
                            .find(|f| f.column == "NickName")
                            .and_then(|f| String::from_utf8_lossy(&f.content).parse().ok());
    
                        username.and_then(|u:String| nickname.map(|n:String| (u, n)))
                    })
                    .collect::<HashMap<_, _>>()
                } else {
                    HashMap::new()
                };
    
                // 构建成员列表
                let members = room_data.members
                    .into_iter()
                    .map(|mut member| {
                        if member.name.as_deref().unwrap_or_default().is_empty() {
                            if let Some(nick) = nick_name_map.get(&member.wxid) {
                                member.name = Some(nick.clone());
                            }
                        }
                        RoomMember {
                            wxid: member.wxid,
                            name: member.name.unwrap_or_default(),
                            state: member.state,
                        }
                    })
                    .collect();
    
                return Ok(Some(members));
            }
        }
    
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_room_member() {}
}
