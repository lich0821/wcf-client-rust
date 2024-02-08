use log::{debug, error, info, warn};
use nng::options::{Options, RecvTimeout};
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
        let exe = env::current_dir()
            .unwrap()
            .join("src\\wcferry\\lib\\wcf.exe");
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
        let _ = match Command::new(exe.to_str().unwrap())
            .creation_flags(0x08000000)
            .args(args)
            .output()
        {
            Ok(output) => output,
            Err(e) => {
                error!("命令行启动失败: {}", e);
                return Err("服务启动失败".into());
            }
        };
        Ok(())
    }

    fn connect(url: &str) -> Result<nng::Socket, Box<dyn std::error::Error>> {
        let client = match nng::Socket::new(nng::Protocol::Pair1) {
            Ok(client) => client,
            Err(e) => {
                error!("Socket创建失败: {}", e);
                return Err("连接服务失败".into());
            }
        };
        match client.set_opt::<RecvTimeout>(Some(Duration::from_millis(5000))) {
            Ok(()) => (),
            Err(e) => {
                error!("连接参数设置失败: {}", e);
                return Err("连接参数设置失败".into());
            }
        };
        match client.set_opt::<nng::options::SendTimeout>(Some(Duration::from_millis(5000))) {
            Ok(()) => (),
            Err(e) => {
                error!("连接参数设置失败: {}", e);
                return Err("连接参数设置失败".into());
            }
        };
        match client.dial(url) {
            Ok(()) => (),
            Err(e) => {
                error!("连接服务失败: {}", e);
                return Err("连接服务失败".into());
            }
        };
        Ok(client)
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.listening.load(Ordering::Relaxed) {
            let _ = self.disable_recv_msg();
            self.listening.store(false, Ordering::Relaxed);
        }
        self.cmd_socket.close();
        let output = Command::new(self.exe.to_str().unwrap())
            .creation_flags(0x08000000)
            .args(["stop"])
            .output();
        let _output = match output {
            Ok(output) => output,
            Err(e) => {
                error!("服务停止失败: {}", e);
                return Err("服务停止失败".into());
            }
        };
        debug!("服务已停止: {}", CMD_URL);
        Ok(())
    }

    fn send_cmd(
        &self,
        req: wcf::Request,
    ) -> Result<Option<wcf::response::Msg>, Box<dyn std::error::Error>> {
        let mut buf = Vec::with_capacity(req.encoded_len());
        match req.encode(&mut buf) {
            Ok(()) => (),
            Err(e) => {
                error!("编码失败: {}", e);
                return Err("编码失败".into());
            }
        };
        let msg = nng::Message::from(&buf[..]);
        let _ = match self.cmd_socket.send(msg) {
            Ok(()) => {}
            Err(e) => {
                error!("消息发送失败: {:?}, {}", e.0, e.1);
                return Err("消息发送失败".into());
            }
        };
        let mut msg = match self.cmd_socket.recv() {
            Ok(msg) => msg,
            Err(e) => {
                error!("消息接收失败: {}", e);
                return Err("消息接收失败".into());
            }
        };
        // 反序列化为prost消息
        let rsp = match wcf::Response::decode(msg.as_slice()) {
            Ok(res) => res,
            Err(e) => {
                error!("解码失败: {}", e);
                return Err("解码失败".into());
            }
        };
        msg.clear();
        Ok(rsp.msg)
    }

    pub fn is_login(self) -> Result<bool, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncIsLogin.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("登录状态命令发送失败: {}", e);
                return Err("登录状态命令发送失败".into());
            }
        };
        if rsp.is_none() {
            return Ok(false);
        }
        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status == 1);
            }
            _ => {
                return Ok(false);
            }
        };
    }

    pub fn get_self_wxid(self) -> Result<String, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetSelfWxid.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("查询 wxid 命令发送失败: {}", e);
                return Err("查询 wxid 命令发送失败".into());
            }
        };
        if rsp.is_none() {
            return Ok("".to_string());
        }
        match rsp.unwrap() {
            wcf::response::Msg::Str(wxid) => {
                return Ok(wxid);
            }
            _ => {
                return Ok("".to_string());
            }
        };
    }

    pub fn get_user_info(self) -> Result<wcf::UserInfo, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetUserInfo.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("获取用户信息命令发送失败: {}", e);
                return Err("获取用户信息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Ui(ui) => {
                return Ok(ui);
            }
            _ => {
                return Err("获取用户信息失败".into());
            }
        };
    }

    pub fn get_contacts(self) -> Result<wcf::RpcContacts, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetContacts.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("获取联系人列表命令发送失败: {}", e);
                return Err("获取联系人列表命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Contacts(contacts) => {
                return Ok(contacts);
            }
            _ => {
                return Err("获取联系人列表失败".into());
            }
        };
    }

    pub fn get_dbs(self) -> Result<wcf::DbNames, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetDbNames.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("获取数据库名称命令发送失败: {}", e);
                return Err("获取数据库名称命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Dbs(dbs) => {
                return Ok(dbs);
            }
            _ => {
                return Err("获取数据库名称失败".into());
            }
        };
    }

    pub fn get_tables(self, db: String) -> Result<wcf::DbTables, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetDbTables.into(),
            msg: Some(wcf::request::Msg::Str(db)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("获取数据表命令发送失败: {}", e);
                return Err("获取数据表命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Tables(tables) => {
                return Ok(tables);
            }
            _ => {
                return Err("获取数据表失败".into());
            }
        };
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
                        if let Some(wcf::response::Msg::Wxmsg(msg)) = rsp.msg {
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
                        // 如果是超时错误，忽略它并继续尝试接收消息
                        debug!("消息接收超时，继续等待...");
                        continue;
                    }
                    Err(e) => {
                        // 对于其他类型的错误，记录警告并返回错误
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

        let req = wcf::Request {
            func: wcf::Functions::FuncEnableRecvTxt.into(),
            msg: Some(wcf::request::Msg::Flag(true)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("启用消息接收命令发送失败: {}", e);
                return Err("启用消息接收命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                if status == 0 {
                    let (tx, rx) = mpsc::sync_channel::<wcf::WxMsg>(100);
                    self.msg_socket = Some(WeChat::connect(MSG_URL).unwrap());
                    self.listening.store(true, Ordering::Relaxed);
                    let mut wc1 = self.clone();
                    let mut wc2 = self.clone();
                    thread::spawn(move || listening_msg(&mut wc1, tx));
                    thread::spawn(move || forward_msg(&mut wc2, cburl, rx));
                }
                return Ok(true);
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

        let req = wcf::Request {
            func: wcf::Functions::FuncDisableRecvTxt.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("停止消息接收命令发送失败: {}", e);
                return Err("停止消息接收命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                // TODO: 处理状态码
                self.msg_socket.clone().unwrap().close();
                self.listening.store(false, Ordering::Relaxed);
                return Ok(status);
            }
            _ => {
                return Err("停止消息接收失败".into());
            }
        };
    }

    pub fn get_msg_types(self) -> Result<wcf::MsgTypes, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetMsgTypes.into(),
            msg: None,
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("获取消息类型命令发送失败: {}", e);
                return Err("获取消息类型命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Types(types) => {
                return Ok(types);
            }
            _ => {
                return Err("获取消息类型失败".into());
            }
        };
    }

    pub fn refresh_pyq(self, id: u64) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncRefreshPyq.into(),
            msg: Some(wcf::request::Msg::Ui64(id)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("刷新朋友圈命令发送失败: {}", e);
                return Err("刷新朋友圈命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("刷新朋友圈失败".into());
            }
        };
    }

    pub fn send_text(self, text: wcf::TextMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncSendTxt.into(),
            msg: Some(wcf::request::Msg::Txt(text)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("发送文本消息命令发送失败: {}", e);
                return Err("发送文本消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("发送文本消息失败".into());
            }
        };
    }

    pub fn send_image(self, img: wcf::PathMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncSendImg.into(),
            msg: Some(wcf::request::Msg::File(img)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("发送图片消息命令发送失败: {}", e);
                return Err("发送图片消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("发送图片消息失败".into());
            }
        };
    }

    pub fn send_file(self, file: wcf::PathMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncSendFile.into(),
            msg: Some(wcf::request::Msg::File(file)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("发送文件消息命令发送失败: {}", e);
                return Err("发送文件消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("发送文件消息失败".into());
            }
        };
    }

    pub fn send_rich_text(self, msg: wcf::RichText) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncSendRichTxt.into(),
            msg: Some(wcf::request::Msg::Rt(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("发送卡片消息命令发送失败: {}", e);
                return Err("发送卡片消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("发送卡片消息失败".into());
            }
        };
    }

    pub fn send_pat_msg(self, msg: wcf::PatMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncSendPatMsg.into(),
            msg: Some(wcf::request::Msg::Pm(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("发送拍一拍消息命令发送失败: {}", e);
                return Err("发送拍一拍消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("发送拍一拍消息失败".into());
            }
        };
    }

    pub fn forward_msg(self, msg: wcf::ForwardMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncForwardMsg.into(),
            msg: Some(wcf::request::Msg::Fm(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("转发消息命令发送失败: {}", e);
                return Err("转发消息命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("转发消息失败".into());
            }
        };
    }

    pub fn save_audio(self, am: wcf::AudioMsg) -> Result<String, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncGetAudioMsg.into(),
            msg: Some(wcf::request::Msg::Am(am)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("保存语音命令发送失败: {}", e);
                return Err("保存语音命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Str(path) => {
                return Ok(path);
            }
            _ => {
                return Err("保存语音失败".into());
            }
        };
    }

    pub fn decrypt_image(self, msg: wcf::DecPath) -> Result<String, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncDecryptImage.into(),
            msg: Some(wcf::request::Msg::Dec(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("解密图片命令发送失败: {}", e);
                return Err("解密图片命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Str(path) => {
                return Ok(path);
            }
            _ => {
                return Err("解密图片失败".into());
            }
        };
    }

    pub fn download_attach(self, msg: wcf::AttachMsg) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncDownloadAttach.into(),
            msg: Some(wcf::request::Msg::Att(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("下载附件命令发送失败: {}", e);
                return Err("下载附件命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("下载附件失败".into());
            }
        };
    }

    pub fn recv_transfer(self, msg: wcf::Transfer) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncRecvTransfer.into(),
            msg: Some(wcf::request::Msg::Tf(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("接收转账命令发送失败: {}", e);
                return Err("接收转账命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("接收转账失败".into());
            }
        };
    }

    pub fn query_sql(self, msg: wcf::DbQuery) -> Result<wcf::DbRows, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncExecDbQuery.into(),
            msg: Some(wcf::request::Msg::Query(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("查询 SQL 命令发送失败: {}", e);
                return Err("查询 SQL 命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Rows(rows) => {
                return Ok(rows);
            }
            _ => {
                return Err("查询 SQL 失败".into());
            }
        };
    }

    pub fn accept_new_friend(
        self,
        msg: wcf::Verification,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncAcceptFriend.into(),
            msg: Some(wcf::request::Msg::V(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("通过好友申请命令发送失败: {}", e);
                return Err("通过好友申请命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("通过好友申请失败".into());
            }
        };
    }

    pub fn add_chatroom_member(
        self,
        msg: wcf::MemberMgmt,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncAddRoomMembers.into(),
            msg: Some(wcf::request::Msg::M(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("添加群成员命令发送失败: {}", e);
                return Err("添加群成员命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("添加群成员失败".into());
            }
        };
    }

    pub fn invite_chatroom_member(
        self,
        msg: wcf::MemberMgmt,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let req = wcf::Request {
            func: wcf::Functions::FuncInvRoomMembers.into(),
            msg: Some(wcf::request::Msg::M(msg)),
        };
        let rsp = match self.send_cmd(req) {
            Ok(res) => res,
            Err(e) => {
                error!("邀请群成员命令发送失败: {}", e);
                return Err("邀请群成员命令发送失败".into());
            }
        };

        match rsp.unwrap() {
            wcf::response::Msg::Status(status) => {
                return Ok(status);
            }
            _ => {
                return Err("邀请群成员失败".into());
            }
        };
    }
}
