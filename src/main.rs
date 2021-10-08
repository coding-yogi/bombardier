use bombardier::{self, cmd, logger};

#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main()  {
    logger::initiate(true);

    let app = cmd::App::new();
    bombardier::process_subcommand(app).await;
}