use base64::encode;
use serde::{Deserialize, Serialize};
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
        AttachMsg, AudioMsg, DbNames, DbQuery, DbTable, DbTables, DecPath, ForwardMsg, MemberMgmt,
        MsgTypes, PatMsg, PathMsg, RichText, RpcContact, RpcContacts, TextMsg, Transfer, UserInfo,
        Verification,
    },
    WeChat,
};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum FieldContent {
    Bytes(Vec<u8>),
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

pub fn get_routes(
    wechat: Arc<Mutex<WeChat>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
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
            DecPath, ForwardMsg, MemberMgmt, MsgTypes, PatMsg, PathMsg, RichText, RpcContact, RpcContacts,
            TextMsg, Transfer, UserInfo, Verification,
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

    fn islogin(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("islogin")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(is_login)
    }

    fn selfwxid(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("selfwxid")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_self_wxid)
    }

    fn userinfo(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("userinfo")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_user_info)
    }

    fn contacts(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("contacts")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_contacts)
    }

    fn dbs(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("dbs")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_dbs)
    }

    fn tables(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!(String / "tables")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_tables)
    }

    fn msgtypes(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("msg-types")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_msg_types)
    }

    fn pyq(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("pyq")
            .and(warp::query::<Id>())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(refresh_pyq)
    }

    fn sendtext(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("text")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(send_text)
    }

    fn sendimage(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("image")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(send_image)
    }

    fn sendfile(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("file")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(send_file)
    }

    fn sendrichtext(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("rich-text")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(send_rich_text)
    }

    fn sendpatmsg(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("pat")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(send_pat_msg)
    }

    fn forwardmsg(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("forward-msg")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(forward_msg)
    }

    fn saveaudio(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("audio")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(save_audio)
    }

    fn saveimage(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("save-image")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(save_image)
    }

    fn recvtransfer(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("receive-transfer")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(recv_transfer)
    }

    fn querysql(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("sql")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(query_sql)
    }

    fn acceptnewfriend(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("accept-new-friend")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(accept_new_friend)
    }

    fn addchatroommember(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("add-chatroom-member")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(add_chatroom_member)
    }

    fn invitechatroommember(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("invite-chatroom-member")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(invite_chatroom_member)
    }

    fn deletechatroommember(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("delete-chatroom-member")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(delete_chatroom_member)
    }

    fn revokemsg(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("revoke-msg")
            .and(warp::post())
            .and(warp::query::<Id>())
            .and(warp::any().map(move || wechat.clone()))
            .and_then(revoke_msg)
    }

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
        return Ok(Box::new(warp::redirect::found(Uri::from_static(
            "/swagger/",
        ))));
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().is_login() {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_self_wxid() {
        Ok(wxid) => ApiResponse {
            status: 0,
            error: None,
            data: Some(wxid),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_user_info() {
        Ok(ui) => ApiResponse {
            status: 0,
            error: None,
            data: Some(ui),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_contacts() {
        Ok(contacts) => ApiResponse {
            status: 0,
            error: None,
            data: Some(contacts),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_dbs() {
        Ok(dbs) => ApiResponse {
            status: 0,
            error: None,
            data: Some(dbs),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_tables(db) {
        Ok(tables) => ApiResponse {
            status: 0,
            error: None,
            data: Some(tables),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_msg_types() {
        Ok(types) => ApiResponse {
            status: 0,
            error: None,
            data: Some(types),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().refresh_pyq(query.id) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().send_text(text) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().send_image(image) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().send_file(file) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().send_rich_text(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().send_pat_msg(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().forward_msg(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 0),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().save_audio(msg) {
        Ok(path) => ApiResponse {
            status: 0,
            error: None,
            data: Some(path),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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

    if status != 0 {
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().recv_transfer(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
}

/// 执行 SQL 查询数据库
#[utoipa::path(
    post,
    tag = "WCF",
    path = "/sql",
    request_body = DbQuery,
    responses(
        (status = 200, body = ApiResponseBool, description = "执行 SQL")
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
                    let fields = r
                        .fields
                        .into_iter()
                        .map(|f| {
                            let utf8 = String::from_utf8(f.content.clone()).unwrap_or_default();
                            let content: FieldContent = match f.r#type {
                                1 => utf8
                                    .parse::<i64>()
                                    .map_or(FieldContent::None, FieldContent::Int),
                                2 => utf8
                                    .parse::<f64>()
                                    .map_or(FieldContent::None, FieldContent::Float),
                                3 => FieldContent::Utf8String(utf8),
                                4 => FieldContent::Base64String(encode(&f.content.clone())),
                                _ => FieldContent::None,
                            };
                            NewField {
                                column: f.column,
                                content,
                            }
                        })
                        .collect::<Vec<_>>();
                    NewRow { fields }
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
pub async fn accept_new_friend(
    msg: Verification,
    wechat: Arc<Mutex<WeChat>>,
) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().accept_new_friend(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
pub async fn add_chatroom_member(
    msg: MemberMgmt,
    wechat: Arc<Mutex<WeChat>>,
) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().add_chatroom_member(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
pub async fn invite_chatroom_member(
    msg: MemberMgmt,
    wechat: Arc<Mutex<WeChat>>,
) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().invite_chatroom_member(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
pub async fn delete_chatroom_member(
    msg: MemberMgmt,
    wechat: Arc<Mutex<WeChat>>,
) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().delete_chatroom_member(msg) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
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
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().revoke_msg(msg.id) {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status == 1),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
}
