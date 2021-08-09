use warp::http::StatusCode;

use std::{
    convert::Infallible,
    sync::Arc
};

use crate::server::servers;

pub async fn start(ctx: Arc<servers::Context>) -> Result<impl warp::Reply, Infallible> {

        //Check if all files received

        //Validate config

        //Check if nodes are available

        //Check if no execution is in process

        //Send the combar message via transmitter


        Ok(StatusCode::CREATED)
}

pub async fn stop(ctx: Arc<servers::Context>) -> Result<impl warp::Reply, Infallible> {
        Ok(StatusCode::OK)
}

pub async fn nodes(ctx: Arc<servers::Context>) -> Result<impl warp::Reply, Infallible> {

        //Get total nodes

        //Get bombarding nodes

        //return respose
        Ok(StatusCode::OK)
}