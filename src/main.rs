use crate::cli::run_cli;
use std::process::exit;

mod api;
mod cli;
mod server;
mod db;
mod user;

fn main() -> Result<(), anyhow::Error> {
    match run_cli() {
        Ok(_) => {
            exit(0);
        }
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}
