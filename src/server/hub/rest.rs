use std::sync::Arc;
use crate::server::{hub::routes,servers};

pub async fn serve(port: u16, ctx: Arc<servers::Context>) {
    let api = routes::bombardier_filters(ctx);
    warp::serve(api).run(([0, 0, 0, 0], port)).await;
}