use std::time::{SystemTime, UNIX_EPOCH};
use warp::reply::{json, Json};
use warp::Filter;
use warp::Rejection;
use warp::Reply;

pub fn hello_world() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("hello" / "world").map(|| json(&"Hello, world!"))
}

pub fn ping() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("ping").map(|| json(&"Pong"))
}

pub fn health() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("health").map(|| json(&"Healthy"))
}

pub fn current_time() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("time").map(|| {
        let now = SystemTime::now();
        json(&format!("{:?}", now))
    })
}

pub fn unix_time() -> impl Filter<Extract = ((Json,),), Error = Rejection> + Clone {
    warp::path("time").map(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        (warp::reply::json(&timestamp),)
    })
}

pub fn get_routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api_v1 = warp::path("api")
        .and(warp::path("v1"))
        .and(hello_world().or(ping()).or(health()).or(current_time()));

    let api_v2 = warp::path("api").and(warp::path("v2")).and(unix_time());

    api_v1.or(api_v2)
}
