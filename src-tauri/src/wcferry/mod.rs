use log::{debug, error, info, warn};
use nng::options::{Options, RecvTimeout, SendTimeout};
use prost::Message;
use reqwest::blocking::Client;
use std::os::windows::process::CommandExt;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, sleep};
use std::{env, path::PathBuf, process::Command, time::Duration, vec};

use crate::wcferry::wcf::WxMsg;

const CMD_URL: &'static str = "tcp://127.0.0.1:10086";
const MSG_URL: &'static str = "tcp://127.0.0.1:10087";

pub mod wcf {
    include!("wcf.rs");
}

use wcf::response::Msg as RspMsg;

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
    ($rsp:expr, Status, $expected:expr, $err_msg:expr) => {
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

#[derive(Clone, Debug)]
pub struct WeChat {
    pub exe: PathBuf,
    pub listening: Arc<AtomicBool>,
    pub cmd_socket: nng::Socket,
    pub msg_socket: Option<nng::Socket>,
}

impl Default for WeChat {
    fn default() -> Self {
        WeChat::new(false, "".to_string())
    }
}

impl WeChat {
    pub fn new(debug: bool, cburl: String) -> Self {
        let exe = env::current_dir().unwrap().join("src\\wcferry\\lib\\wcf.exe");
        let _ = WeChat::start(exe.clone(), debug);
        let cmd_socket = WeChat::connect(&CMD_URL).unwrap();
        let mut wc = WeChat {
            exe: exe,
            listening: Arc::new(AtomicBool::new(false)),
            cmd_socket,
            msg_socket: None,
        };
        info!("等待微信登录...");
        while !wc.clone().is_login().unwrap() {
            sleep(Duration::from_secs(1));
        }
        let _ = wc.enable_recv_msg(cburl);
        wc
    }

    fn start(exe: PathBuf, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = vec!["start", "10086"];
        if debug {
            args.push("debug");
        }
        debug!("exe: {}, debug: {}", exe.clone().to_str().unwrap(), debug);
        let _ = try_cmd!(
            Command::new(exe.to_str().unwrap_or_default())
                .creation_flags(0x08000000)
                .args(&args)
                .output(),
            "wcf.exe 启动失败"
        );
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
        try_cmd!(
            Command::new(self.exe.to_str().unwrap())
                .creation_flags(0x08000000)
                .args(["stop"])
                .output(),
            "服务停止失败"
        );
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

    pub fn is_login(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncIsLogin);
        let rsp = try_cmd!(self.send_cmd(req), "登录状态命令发送失败");
        process_response!(rsp, Status, 1, "获取登录状态失败")
    }

    pub fn get_self_wxid(&self) -> Result<String, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetSelfWxid);
        let rsp = try_cmd!(self.send_cmd(req), "查询 wxid 命令发送失败");
        process_response!(rsp, Str, "获取 wxid 失败")
    }

    pub fn get_user_info(self) -> Result<wcf::UserInfo, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetUserInfo);
        let rsp = try_cmd!(self.send_cmd(req), "获取用户信息命令发送失败");
        process_response!(rsp, Ui, "获取用户信息失败")
    }

    pub fn get_contacts(self) -> Result<wcf::RpcContacts, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetContacts);
        let rsp = try_cmd!(self.send_cmd(req), "获取联系人列表命令发送失败");
        process_response!(rsp, Contacts, "获取联系人列表失败")
    }

    pub fn get_dbs(self) -> Result<wcf::DbNames, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetDbNames);
        let rsp = try_cmd!(self.send_cmd(req), "获取数据库名称命令发送失败");
        process_response!(rsp, Dbs, "获取数据库名称失败")
    }

    pub fn get_tables(self, db: String) -> Result<wcf::DbTables, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetDbTables, wcf::request::Msg::Str(db));
        let rsp = try_cmd!(self.send_cmd(req), "获取数据表命令发送失败");
        process_response!(rsp, Tables, "获取数据表失败")
    }

    pub fn enable_recv_msg(&mut self, cburl: String) -> Result<bool, Box<dyn std::error::Error>> {
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

        fn forward_msg(wechat: &mut WeChat, cburl: String, rx: Receiver<WxMsg>) {
            let mut cb_client = None;
            if !cburl.is_empty() {
                cb_client = Some(Client::new());
            }
            while wechat.listening.load(Ordering::Relaxed) {
                match rx.recv() {
                    Ok(msg) => {
                        if let Some(client) = &cb_client {
                            match client.post(cburl.clone()).json(&msg).send() {
                                Ok(rsp) => {
                                    if !rsp.status().is_success() {
                                        error!("转发消息失败，状态码: {}", rsp.status().as_str());
                                    }
                                }
                                Err(e) => {
                                    error!("转发消息失败：{}", e);
                                }
                            }
                        } else {
                            info!("收到消息:\n{:?}", msg);
                        };
                    }
                    Err(e) => {
                        error!("消息出队失败: {}", e);
                    }
                }
            }
        }

        if self.listening.load(Ordering::Relaxed) {
            warn!("已经启用消息接收");
            return Ok(true);
        }

        let req = create_request!(wcf::Functions::FuncEnableRecvTxt, wcf::request::Msg::Flag(true));
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
                    thread::spawn(move || forward_msg(&mut wc2, cburl, rx));
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

        let req = create_request!(wcf::Functions::FuncDisableRecvTxt);
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

    pub fn get_msg_types(self) -> Result<wcf::MsgTypes, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetMsgTypes);
        let rsp = try_cmd!(self.send_cmd(req), "获取消息类型命令发送失败");
        process_response!(rsp, Types, "获取消息类型失败")
    }

    pub fn refresh_pyq(self, id: u64) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncRefreshPyq, wcf::request::Msg::Ui64(id));
        let rsp = try_cmd!(self.send_cmd(req), "刷新朋友圈命令发送失败");
        process_response!(rsp, Status, 1, "刷新朋友圈失败")
    }

    pub fn send_text(self, text: wcf::TextMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncSendTxt, wcf::request::Msg::Txt(text));
        let rsp = try_cmd!(self.send_cmd(req), "发送文本消息命令发送失败");
        process_response!(rsp, Status, 0, "发送文本消息失败")
    }

    pub fn send_image(self, img: wcf::PathMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncSendImg, wcf::request::Msg::File(img));
        let rsp = try_cmd!(self.send_cmd(req), "发送图片消息命令发送失败");
        process_response!(rsp, Status, 0, "发送图片消息失败")
    }

    pub fn send_file(self, file: wcf::PathMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncSendFile, wcf::request::Msg::File(file));
        let rsp = try_cmd!(self.send_cmd(req), "发送文件消息命令发送失败");
        process_response!(rsp, Status, 0, "发送文件消息失败")
    }

    pub fn send_rich_text(self, msg: wcf::RichText) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncSendRichTxt, wcf::request::Msg::Rt(msg));
        let rsp = try_cmd!(self.send_cmd(req), "发送卡片消息命令发送失败");
        process_response!(rsp, Status, 0, "发送卡片消息失败")
    }

    pub fn send_pat_msg(self, msg: wcf::PatMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncSendPatMsg, wcf::request::Msg::Pm(msg));
        let rsp = try_cmd!(self.send_cmd(req), "发送拍一拍消息命令发送失败");
        process_response!(rsp, Status, 1, "发送拍一拍消息失败")
    }

    pub fn forward_msg(self, msg: wcf::ForwardMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncForwardMsg, wcf::request::Msg::Fm(msg));
        let rsp = try_cmd!(self.send_cmd(req), "转发消息命令发送失败");
        process_response!(rsp, Status, 1, "转发消息失败")
    }

    pub fn save_audio(self, am: wcf::AudioMsg) -> Result<String, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncGetAudioMsg, wcf::request::Msg::Am(am));
        let rsp = try_cmd!(self.send_cmd(req), "保存语音命令发送失败");
        process_response!(rsp, Str, "保存语音失败")
    }

    pub fn decrypt_image(self, msg: wcf::DecPath) -> Result<String, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncDecryptImage, wcf::request::Msg::Dec(msg));
        let rsp = try_cmd!(self.send_cmd(req), "解密图片命令发送失败");
        process_response!(rsp, Str, "解密图片失败")
    }

    pub fn download_attach(self, msg: wcf::AttachMsg) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncDownloadAttach, wcf::request::Msg::Att(msg));
        let rsp = try_cmd!(self.send_cmd(req), "下载附件命令发送失败");
        process_response!(rsp, Status, 0, "下载附件失败")
    }

    pub fn recv_transfer(self, msg: wcf::Transfer) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncRecvTransfer, wcf::request::Msg::Tf(msg));
        let rsp = try_cmd!(self.send_cmd(req), "接收转账命令发送失败");
        process_response!(rsp, Status, 1, "接收转账失败")
    }

    pub fn query_sql(self, msg: wcf::DbQuery) -> Result<wcf::DbRows, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncExecDbQuery, wcf::request::Msg::Query(msg));
        let rsp = try_cmd!(self.send_cmd(req), "查询 SQL 命令发送失败");
        process_response!(rsp, Rows, "查询 SQL 失败")
    }

    pub fn accept_new_friend(self, msg: wcf::Verification) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncAcceptFriend, wcf::request::Msg::V(msg));
        let rsp = try_cmd!(self.send_cmd(req), "通过好友申请命令发送失败");
        process_response!(rsp, Status, 1, "通过好友申请失败")
    }

    pub fn add_chatroom_member(self, msg: wcf::MemberMgmt) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncAddRoomMembers, wcf::request::Msg::M(msg));
        let rsp = try_cmd!(self.send_cmd(req), "添加群成员命令发送失败");
        process_response!(rsp, Status, 1, "添加群成员失败")
    }

    pub fn invite_chatroom_member(self, msg: wcf::MemberMgmt) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncInvRoomMembers, wcf::request::Msg::M(msg));
        let rsp = try_cmd!(self.send_cmd(req), "邀请群成员命令发送失败");
        process_response!(rsp, Status, 1, "邀请群成员失败")
    }

    pub fn delete_chatroom_member(self, msg: wcf::MemberMgmt) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncDelRoomMembers, wcf::request::Msg::M(msg));
        let rsp = try_cmd!(self.send_cmd(req), "删除群成员命令发送失败");
        process_response!(rsp, Status, 1, "删除群成员失败")
    }

    pub fn revoke_msg(self, id: u64) -> Result<bool, Box<dyn std::error::Error>> {
        let req = create_request!(wcf::Functions::FuncRevokeMsg, wcf::request::Msg::Ui64(id));
        let rsp = try_cmd!(self.send_cmd(req), "撤回消息命令发送失败");
        process_response!(rsp, Status, 1, "撤回消息失败")
    }
}
