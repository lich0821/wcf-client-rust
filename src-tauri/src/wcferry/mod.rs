use log::{debug, error, info};
use nng::options::{Options, RecvTimeout};
use prost::Message;
use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::thread::sleep;
use std::{env, path::PathBuf, process::Command, time::Duration, vec};

const DEFAULT_URL: &'static str = "tcp://127.0.0.1:10086";

pub mod wcf {
    include!("wcf.rs");
}

#[derive(Clone, Debug)]
pub struct WeChat {
    pub url: String,
    pub exe: PathBuf,
    pub debug: bool,
    pub socket: nng::Socket,
    pub listening: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct UserInfo {
    pub wxid: String,
    pub name: String,
    pub mobile: String,
    pub home: String,
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
        let socket = WeChat::connect(&DEFAULT_URL).unwrap();
        let wc = WeChat {
            url: String::from(DEFAULT_URL),
            exe: exe,
            debug,
            socket,
            listening: false,
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
        self.socket.close();
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
        let _ = match self.socket.send(msg) {
            Ok(()) => {}
            Err(e) => {
                error!("消息发送失败: {:?}, {}", e.0, e.1);
                return Err("消息发送失败".into());
            }
        };
        let mut msg = match self.socket.recv() {
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
}
