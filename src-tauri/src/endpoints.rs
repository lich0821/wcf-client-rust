use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
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
        AudioMsg, DbNames, DbTable, DbTables, ForwardMsg, MsgTypes, PatMsg, PathMsg, RichText,
        RpcContact, RpcContacts, TextMsg, UserInfo,
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

pub fn get_routes(
    wechat: Arc<Mutex<WeChat>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let config = Arc::new(Config::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        paths(is_login, get_self_wxid, get_user_info, get_contacts, get_dbs, get_tables, get_msg_types, save_audio,
            refresh_pyq, send_text, send_image, send_file, send_rich_text, send_pat_msg, forward_msg),
        components(schemas(
            ApiResponse<bool>, ApiResponse<String>, AudioMsg, DbNames, DbTable, DbTables, ForwardMsg, MsgTypes, PatMsg,
            PathMsg, RichText, RpcContact, RpcContacts, TextMsg, UserInfo,
        )),
        tags((name = "WCF", description = "玩微信的接口"))
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
