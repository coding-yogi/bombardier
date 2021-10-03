use bombardier::{self, cmd, logger};

#[tokio::main]
async fn main()  {
    logger::initiate(true);

    let app = cmd::App::new();
    bombardier::process_subcommand(app).await;
}