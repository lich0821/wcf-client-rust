use log::{debug, error, info, warn};
use nng::options::{Options, RecvTimeout};
use prost::Message;
use std::os::windows::process::CommandExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, sleep};
use std::{env, path::PathBuf, process::Command, time::Duration, vec};

const CMD_URL: &'static str = "tcp://127.0.0.1:10086";
const MSG_URL: &'static str = "tcp://127.0.0.1:10087";

pub mod wcf {
    include!("wcf.rs");
}

#[derive(Clone, Debug)]
pub struct WeChat {
    pub url: String,
    pub exe: PathBuf,
    pub debug: bool,
    pub listening: Arc<AtomicBool>,
    pub cmd_socket: nng::Socket,
    pub msg_socket: Option<nng::Socket>,
}

impl Default for WeChat {
    fn default() -> Self {
        WeChat::new(false)
    }
}

impl WeChat {
    pub fn new(debug: bool) -> Self {
        let exe = env::current_dir()
            .unwrap()
            .join("src\\wcferry\\lib\\wcf.exe");
        let _ = WeChat::start(exe.clone(), debug);
        let cmd_socket = WeChat::connect(&CMD_URL).unwrap();
        let wc = WeChat {
            url: String::from(CMD_URL),
            exe: exe,
            debug,
            listening: Arc::new(AtomicBool::new(false)),
            cmd_socket,
            msg_socket: None,
        };
        info!("等待微信登录...");
        while !wc.clone().is_login().unwrap() {
            sleep(Duration::from_secs(1));
        }
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
        debug!("服务已停止: {}", self.url);
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
                return Ok(1 == status);
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

    pub fn enable_recv_msg(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        fn listening_msg(wechat: &mut WeChat) {
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
                            // TODO: 转发消息
                            info!("接收到消息: {:?}", msg);
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
                    self.msg_socket = Some(WeChat::connect(MSG_URL).unwrap());
                    self.listening.store(true, Ordering::Relaxed);
                    let mut wc = self.clone();
                    thread::spawn(move || listening_msg(&mut wc));
                }
                return Ok(true);
            }
            _ => {
                return Err("启用消息接收失败".into());
            }
        };
    }

    pub fn disable_recv_msg(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.listening.load(Ordering::Relaxed) {
            return Ok(true);
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
            wcf::response::Msg::Status(_status) => {
                // TODO: 处理状态码
                self.msg_socket.clone().unwrap().close();
                self.listening.store(false, Ordering::Relaxed);
                return Ok(true);
            }
            _ => {
                return Err("停止消息接收失败".into());
            }
        };
    }
}
