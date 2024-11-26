use crate::cli::run_cli;

mod cli;
mod server;
mod db;
mod api;

fn main() -> Result<(), anyhow::Error>{
    run_cli()
}
