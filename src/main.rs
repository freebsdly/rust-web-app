use crate::cli::run_cli;

mod api;
mod cli;
mod server;
mod db;
mod user;
mod log;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
   run_cli().await
}
