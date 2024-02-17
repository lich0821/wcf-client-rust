use base64::encode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::Config;
use warp::reply::Json;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};

use crate::wcferry::{
    wcf::{
        AttachMsg, AudioMsg, DbNames, DbQuery, DbTable, DbTables, DecPath, ForwardMsg, MemberMgmt, MsgTypes, PatMsg,
        PathMsg, RichText, RpcContact, RpcContacts, TextMsg, Transfer, UserInfo, Verification,
    },
    WeChat,
};

#[macro_export]
macro_rules! wechat_api_handler {
    ($wechat:expr, $handler:expr, $desc:expr) => {{
        let wechat = $wechat.lock().unwrap();
        let result: Result<_, _> = $handler(&*wechat);
        match result {
            Ok(data) => Ok(warp::reply::json(&ApiResponse {
                status: 0,
                error: None,
                data: Some(data),
            })),
            Err(error) => Ok(warp::reply::json(&ApiResponse::<()> {
                status: 1,
                error: Some(format!("{}失败: {}", $desc, error)),
                data: None,
            })),
        }
    }};
    ($wechat:expr, $handler:expr, $param:expr, $desc:expr) => {{
        let wechat = $wechat.lock().unwrap();
        let result: Result<_, _> = $handler(&*wechat, $param);
        match result {
            Ok(data) => Ok(warp::reply::json(&ApiResponse {
                status: 0,
                error: None,
                data: Some(data),
            })),
            Err(error) => Ok(warp::reply::json(&ApiResponse::<()> {
                status: 1,
                error: Some(format!("{}失败: {}", $desc, error)),
                data: None,
            })),
        }
    }};
}

#[macro_export]
macro_rules! build_route_fn {
    ($func_name:ident, GET $path:expr, $handler:expr, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path($path)
                .and(warp::get())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
    ($func_name:ident, GET $path:expr, $handler:expr, PATH $param_type:ty, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path::param::<$param_type>()
                .and(warp::path($path))
                .and(warp::get())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
    ($func_name:ident, GET $path:expr, $handler:expr, QUERY $param_type:ty, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path($path)
                .and(warp::get())
                .and(warp::query::<$param_type>())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
    ($func_name:ident, POST $path:expr, $handler:expr, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path($path)
                .and(warp::post())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
    ($func_name:ident, POST $path:expr, $handler:expr, QUERY $param_type:ty, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path($path)
                .and(warp::post())
                .and(warp::query::<$param_type>())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
    ($func_name:ident, POST $path:expr, $handler:expr, JSON, $wechat:expr) => {
        pub fn $func_name(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
            warp::path($path)
                .and(warp::post())
                .and(warp::body::json())
                .and(warp::any().map(move || wechat.clone()))
                .and_then($handler)
        }
    };
}

#[derive(Serialize, ToSchema, Clone)]
#[aliases(ApiResponseBool = ApiResponse<bool>,
    ApiResponseString = ApiResponse<String>,
    ApiResponseUserInfo = ApiResponse<UserInfo>,
    ApiResponseContacts = ApiResponse<RpcContacts>,
    ApiResponseDbNames = ApiResponse<DbNames>,
    ApiResponseMsgTypes = ApiResponse<MsgTypes>,
    ApiResponseDbTables = ApiResponse<DbTables>)]
struct ApiResponse<T>
where
    T: Serialize,
{
    status: u16,
    error: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Id {
    id: u64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct Image {
    /// 消息里的 id
    id: u64,
    /// 消息里的 extra
    extra: String,
    /// 存放目录，不存在则失败；没权限，亦失败
    #[schema(example = "C:/")]
    dir: String,
    /// 超时时间，单位秒
    #[schema(example = 10)]
    timeout: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(untagged)]
pub enum FieldContent {
    Int(i64),
    Float(f64),
    Utf8String(String),
    Base64String(String),
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewField {
    /// 字段名称
    pub column: String,
    /// 字段内容
    pub content: FieldContent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewRow {
    pub fields: Vec<NewField>,
}

pub fn get_routes(wechat: Arc<Mutex<WeChat>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let config = Arc::new(Config::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        info(description = "<a href='https://github.com/lich0821/WeChatFerry'>WeChatFerry</a> 一个玩微信的工具。<table align='left'><tbody><tr><td align='center'><img width='160' alt='碲矿' src='https://s2.loli.net/2023/09/25/fub5VAPSa8srwyM.jpg'><div align='center' width='200'>后台回复 <code>WCF</code> 加群交流</div></td><td align='center'><img width='160' alt='赞赏' src='https://s2.loli.net/2023/09/25/gkh9uWZVOxzNPAX.jpg'><div align='center' width='200'>如果你觉得有用</div></td><td width='20%'></td><td width='20%'></td><td width='20%'></td></tr></tbody></table>"),
        paths(is_login, get_self_wxid, get_user_info, get_contacts, get_dbs, get_tables, get_msg_types, save_audio,
            refresh_pyq, send_text, send_image, send_file, send_rich_text, send_pat_msg, forward_msg, save_image,
            recv_transfer, query_sql, accept_new_friend, add_chatroom_member, invite_chatroom_member,
            delete_chatroom_member, revoke_msg),
        components(schemas(
            ApiResponse<bool>, ApiResponse<String>, AttachMsg, AudioMsg, DbNames, DbQuery, DbTable, DbTables,
            DecPath, FieldContent, ForwardMsg, Image, MemberMgmt, MsgTypes, PatMsg, PathMsg, RichText, RpcContact,
            RpcContacts, TextMsg, Transfer, UserInfo, Verification,
        )),
        tags((name = "WCF", description = "玩微信的接口")),
    )]
    struct ApiDoc;

    let api_doc = warp::path("api-doc.json")
        .and(warp::get())
        .map(|| warp::reply::json(&ApiDoc::openapi()));

    let swagger_ui = warp::path("swagger")
        .and(warp::get())
        .and(warp::path::full())
        .and(warp::path::tail())
        .and(warp::any().map(move || config.clone()))
        .and_then(serve_swagger);

    build_route_fn!(islogin, GET "islogin", is_login, wechat);
    build_route_fn!(selfwxid, GET "selfwxid", get_self_wxid, wechat);
    build_route_fn!(userinfo, GET "userinfo", get_user_info, wechat);
    build_route_fn!(contacts, GET "contacts", get_contacts, wechat);
    build_route_fn!(dbs, GET "dbs", get_dbs, wechat);
    build_route_fn!(tables, GET "tables", get_tables, PATH String, wechat);
    build_route_fn!(msgtypes, GET "msg-types", get_msg_types, wechat);
    build_route_fn!(pyq, GET "pyq", refresh_pyq, QUERY Id, wechat);
    build_route_fn!(sendtext, POST "text", send_text, JSON, wechat);
    build_route_fn!(sendimage, POST "image", send_image, JSON, wechat);
    build_route_fn!(sendfile, POST "file", send_file, JSON, wechat);
    build_route_fn!(sendrichtext, POST "rich-text", send_rich_text, JSON, wechat);
    build_route_fn!(sendpatmsg, POST "pat", send_pat_msg, JSON, wechat);
    build_route_fn!(forwardmsg, POST "forward-msg", forward_msg, JSON, wechat);
    build_route_fn!(saveaudio, POST "audio", save_audio, JSON, wechat);
    build_route_fn!(saveimage, POST "save-image", save_image, JSON, wechat);
    build_route_fn!(recvtransfer, POST "receive-transfer", recv_transfer, JSON, wechat);
    build_route_fn!(querysql, POST "sql", query_sql, JSON, wechat);
    build_route_fn!(acceptnewfriend, POST "accept-new-friend", accept_new_friend, JSON, wechat);
    build_route_fn!(addchatroommember, POST "add-chatroom-member", add_chatroom_member, JSON, wechat);
    build_route_fn!(invitechatroommember, POST "invite-chatroom-member", invite_chatroom_member, JSON, wechat);
    build_route_fn!(deletechatroommember, POST "delete-chatroom-member", delete_chatroom_member, JSON, wechat);
    build_route_fn!(revokemsg, POST "revoke-msg", revoke_msg, QUERY Id, wechat);

    api_doc
        .or(swagger_ui)
        .or(islogin(wechat.clone()))
        .or(selfwxid(wechat.clone()))
        .or(userinfo(wechat.clone()))
        .or(contacts(wechat.clone()))
        .or(dbs(wechat.clone()))
        .or(tables(wechat.clone()))
        .or(msgtypes(wechat.clone()))
        .or(pyq(wechat.clone()))
        .or(sendtext(wechat.clone()))
        .or(sendimage(wechat.clone()))
        .or(sendfile(wechat.clone()))
        .or(sendrichtext(wechat.clone()))
        .or(sendpatmsg(wechat.clone()))
        .or(forwardmsg(wechat.clone()))
        .or(saveaudio(wechat.clone()))
        .or(saveimage(wechat.clone()))
        .or(recvtransfer(wechat.clone()))
        .or(querysql(wechat.clone()))
        .or(acceptnewfriend(wechat.clone()))
        .or(addchatroommember(wechat.clone()))
        .or(invitechatroommember(wechat.clone()))
        .or(deletechatroommember(wechat.clone()))
        .or(revokemsg(wechat.clone()))
}

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<Config<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
    if full_path.as_str() == "/swagger" {
        return Ok(Box::new(warp::redirect::found(Uri::from_static("/swagger/"))));
    }

    let path = tail.as_str();
    match utoipa_swagger_ui::serve(path, config) {
        Ok(file) => {
            if let Some(file) = file {
                Ok(Box::new(
                    Response::builder()
                        .header("Content-Type", file.content_type)
                        .body(file.bytes),
                ))
            } else {
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
        }
        Err(error) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error.to_string()),
        )),
    }
}

/// 查询登录状态
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/islogin",
    responses(
        (status = 200, body = ApiResponseBool, description = "查询微信登录状态")
    )
)]
pub async fn is_login(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::is_login, "查询微信登录状态")
}

/// 查询登录 wxid
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/selfwxid",
    responses(
        (status = 200, body = ApiResponseString, description = "返回登录账户 wxid")
    )
)]
pub async fn get_self_wxid(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_self_wxid, "查询登录 wxid ")
}

/// 获取登录账号信息
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/userinfo",
    responses(
        (status = 200, body = ApiResponseUserInfo, description = "返回登录账户用户信息")
    )
)]
pub async fn get_user_info(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_user_info, "获取登录账号信息")
}

/// 获取所有联系人
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/contacts",
    responses(
        (status = 200, body = ApiResponseContacts, description = "查询所有联系人，包括服务号、公众号、群聊等")
    )
)]
pub async fn get_contacts(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_contacts, "获取所有联系人")
}

/// 获取所有可查询数据库
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/dbs",
    responses(
        (status = 200, body = ApiResponseDbNames, description = "查询所有可用数据库")
    )
)]
pub async fn get_dbs(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_dbs, "获取所有可查询数据库")
}

/// 查询数据库下的表信息
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/{db}/tables",
    params(
        ("db" = String, Path, description = "目标数据库")
    ),
    responses(
        (status = 200, body = ApiResponseDbTables, description = "返回数据库表信息")
    )
)]
pub async fn get_tables(db: String, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_tables, db, "查询数据库下的表信息")
}

/// 获取消息类型枚举
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/msg-types",
    responses(
        (status = 200, body = ApiResponseMsgTypes, description = "返回消息类型")
    )
)]
pub async fn get_msg_types(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::get_msg_types, "获取消息类型枚举")
}

/// 刷新朋友圈（在消息回调中查看）
#[utoipa::path(
    get,
    tag = "WCF",
    path = "/pyq",
    params(("id"=u64, Query, description = "开始 id，0 为最新页")),
    responses(
        (status = 200, body = ApiResponseBool, description = "刷新朋友圈（从消息回调中查看）")
    )
)]
pub async fn refresh_pyq(query: Id, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::refresh_pyq, query.id, "刷新朋友圈")
}

/// 发送文本消息
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/text",
    request_body = TextMsg,
    responses(
        (status = 200, body = ApiResponseBool, description = "发送文本消息")
    )
)]
pub async fn send_text(text: TextMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::send_text, text, "发送文本消息")
}

/// 发送图片
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/image",
    request_body = PathMsg,
    responses(
        (status = 200, body = ApiResponseBool, description = "发送图片消息")
    )
)]
pub async fn send_image(image: PathMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::send_image, image, "发送图片消息")
}

/// 发送文件
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/file",
    request_body = PathMsg,
    responses(
        (status = 200, body = ApiResponseBool, description = "发送文件消息")
    )
)]
pub async fn send_file(file: PathMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::send_file, file, "发送文件消息")
}

/// 发送卡片消息
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/rich-text",
    request_body = RichText,
    responses(
        (status = 200, body = ApiResponseBool, description = "发送卡片消息")
    )
)]
pub async fn send_rich_text(msg: RichText, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::send_rich_text, msg, "发送卡片消息")
}

/// 拍一拍
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/pat",
    request_body = PatMsg,
    responses(
        (status = 200, body = ApiResponseBool, description = "发送拍一拍消息")
    )
)]
pub async fn send_pat_msg(msg: PatMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::send_pat_msg, msg, "发送拍一拍消息")
}

/// 转发消息
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/forward-msg",
    request_body = ForwardMsg,
    responses(
        (status = 200, body = ApiResponseBool, description = "转发消息")
    )
)]
pub async fn forward_msg(msg: ForwardMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::forward_msg, msg, "转发消息")
}

/// 保存语音
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/audio",
    request_body = AudioMsg,
    responses(
        (status = 200, body = ApiResponseString, description = "保存语音消息")
    )
)]
pub async fn save_audio(msg: AudioMsg, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::save_audio, msg, "保存语音")
}

/// 保存图片
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/save-image",
    request_body = Image,
    responses(
        (status = 200, body = ApiResponseString, description = "保存图片")
    )
)]
pub async fn save_image(msg: Image, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    let wc = wechat.lock().unwrap();
    let handle_error = |error_message: &str| -> Result<Json, Infallible> {
        Ok(warp::reply::json(&ApiResponse::<String> {
            status: 1,
            error: Some(error_message.to_string()),
            data: None,
        }))
    };

    let att = AttachMsg {
        id: msg.id,
        thumb: "".to_string(),
        extra: msg.extra.clone(),
    };

    let status = match wc.clone().download_attach(att) {
        Ok(status) => status,
        Err(error) => return handle_error(&error.to_string()),
    };

    if !status {
        return handle_error("下载失败");
    }

    let mut counter = 0;
    loop {
        if counter >= msg.timeout {
            break;
        }
        match wc.clone().decrypt_image(DecPath {
            src: msg.extra.clone(),
            dst: msg.dir.clone(),
        }) {
            Ok(path) => {
                if path.is_empty() {
                    counter += 1;
                    sleep(Duration::from_secs(1));
                    continue;
                }
                return Ok(warp::reply::json(&ApiResponse {
                    status: 0,
                    error: None,
                    data: Some(path),
                }));
            }
            Err(error) => return handle_error(&error.to_string()),
        };
    }
    return handle_error("下载超时");
}

/// 接收转账
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/receive-transfer",
    request_body = Transfer,
    responses(
        (status = 200, body = ApiResponseBool, description = "接收转账")
    )
)]
pub async fn recv_transfer(msg: Transfer, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::recv_transfer, msg, "接收转账")
}

/// 执行 SQL 查询数据库
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/sql",
    request_body = DbQuery,
    responses(
        (status = 200, body = Vec<HashMap<String, FieldContent>>, description = "执行 SQL")
    )
)]
pub async fn query_sql(msg: DbQuery, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().query_sql(msg) {
        Ok(origin) => {
            let rows = origin
                .rows
                .into_iter()
                .map(|r| {
                    let mut row_map = HashMap::new();
                    for f in r.fields {
                        let utf8 = String::from_utf8(f.content.clone()).unwrap_or_default();
                        let content: FieldContent = match f.r#type {
                            1 => utf8.parse::<i64>().map_or(FieldContent::None, FieldContent::Int),
                            2 => utf8.parse::<f64>().map_or(FieldContent::None, FieldContent::Float),
                            3 => FieldContent::Utf8String(utf8),
                            4 => FieldContent::Base64String(encode(&f.content.clone())),
                            _ => FieldContent::None,
                        };
                        row_map.insert(f.column, content);
                    }
                    row_map
                })
                .collect::<Vec<_>>();

            ApiResponse {
                status: 0,
                error: None,
                data: Some(rows),
            }
        }
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
}

/// 通过好友申请
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/accept-new-friend",
    request_body = Verification,
    responses(
        (status = 200, body = ApiResponseBool, description = "通过好友申请")
    )
)]
pub async fn accept_new_friend(msg: Verification, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::accept_new_friend, msg, "通过好友申请")
}

/// 添加群成员
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/add-chatroom-member",
    request_body = MemberMgmt,
    responses(
        (status = 200, body = ApiResponseBool, description = "添加群成员")
    )
)]
pub async fn add_chatroom_member(msg: MemberMgmt, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::add_chatroom_member, msg, "添加群成员")
}

/// 邀请群成员
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/invite-chatroom-member",
    request_body = MemberMgmt,
    responses(
        (status = 200, body = ApiResponseBool, description = "邀请群成员")
    )
)]
pub async fn invite_chatroom_member(msg: MemberMgmt, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::invite_chatroom_member, msg, "邀请群成员")
}

/// 删除群成员（踢人）
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/delete-chatroom-member",
    request_body = MemberMgmt,
    responses(
        (status = 200, body = ApiResponseBool, description = "删除群成员")
    )
)]
pub async fn delete_chatroom_member(msg: MemberMgmt, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::delete_chatroom_member, msg, "删除群成员")
}

/// 撤回消息
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/revoke-msg",
    params(("id"=u64, Query, description = "待撤回消息 id")),
    responses(
        (status = 200, body = ApiResponseBool, description = "撤回消息")
    )
)]
pub async fn revoke_msg(msg: Id, wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    wechat_api_handler!(wechat, WeChat::revoke_msg, msg.id, "撤回消息")
}
