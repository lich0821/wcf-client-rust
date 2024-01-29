use serde::Serialize;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::Config;
use warp::reply::Json;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};

use crate::wcferry::WeChat;

#[derive(Serialize, ToSchema, Clone)]
#[aliases(ApiResponseBool = ApiResponse<bool>)]
struct ApiResponse<T>
where
    T: Serialize,
{
    status: u16,
    error: Option<String>,
    data: T,
}

pub fn get_routes(
    wechat: Arc<Mutex<WeChat>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let config = Arc::new(Config::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(paths(is_login), components(schemas(ApiResponse<String>, ApiResponse<bool>)))]
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

    api_doc.or(swagger_ui).or(islogin(wechat))
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
    path = "/islogin",
    responses(
        (status = 200, body = ApiResponseBool, description = "Returns login status")
    )
)]
pub async fn is_login(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let status = wechat.clone().is_login().unwrap();
    let rsp = ApiResponse {
        status: 0,
        error: None,
        data: status,
    };
    Ok(warp::reply::json(&rsp))
}
