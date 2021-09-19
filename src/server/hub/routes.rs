use warp::Filter;

use std::sync::Arc;

use crate::server::{
    hub::api,
    servers
};

pub fn bombardier_filters(ctx: Arc<servers::Context>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    start_execution(ctx.clone())
        .or(stop_execution(ctx.clone()))
        .or(get_available_nodes(ctx))
}

pub fn start_execution(ctx: Arc<servers::Context>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("bombardier" / "v1" / "bombard")
        .and(warp::post())
        .and(with_context(ctx))
        .and(warp::multipart::form())
        .and_then(api::start)
}

pub fn stop_execution(ctx: Arc<servers::Context>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("bombardier" / "v1" / "stop")
        .and(warp::post())
        .and(with_context(ctx))
        .and_then(api::stop)
}

pub fn get_available_nodes(ctx: Arc<servers::Context>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path!("bombardier" / "v1" / "nodes")
        .and(warp::get())
        .and(with_context(ctx))
        .and_then(api::nodes)
}

fn with_context(ctx: Arc<servers::Context>) ->
impl Filter<Extract = (Arc<servers::Context>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || ctx.clone())
}