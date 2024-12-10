use std::time::Duration;
use crate::server::{ServiceManager, ServiceManagerArgs};
use clap::{Args, Parser, Subcommand};
use config::Config;
use serde::Deserialize;
use tokio::{select, signal, time};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(arg_required_else_help(true))]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<SubCommands>,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// Start the server
    Start(StartServerArgs),
}

#[derive(Args, Debug)]
pub struct StartServerArgs {
    /// Configuration file path
    #[arg(short, long, default_value = "etc/web-app.yaml")]
    pub path: String,
    #[arg(short, long)]
    pub graceful_shutdown: bool,
}

pub fn parse_settings<'a, T: Deserialize<'a>>(path: &str) -> Result<T, anyhow::Error> {
    let settings = Config::builder()
        .add_source(config::File::with_name(path))
        .build()?;

    Ok(settings.try_deserialize::<T>()?)
}

pub async fn start_server(args: StartServerArgs) -> Result<(), anyhow::Error> {
    info!("starting server using configuration: {:?}", args.path);
    let server_args = parse_settings::<ServiceManagerArgs>(&*args.path)?;
    let server = ServiceManager::new(server_args.clone())?;
    server.start()?;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    let with_graceful_shutdown = args.graceful_shutdown.clone();
    select! {
        _ = ctrl_c => {
            info!("receive ctrl_c to shutting down server");
            if with_graceful_shutdown {
                server.stop()?
            } else {
                server.stop_force()?
            }
        },
        _ = terminate => {
            info!("signal handler exited unexpectedly");
            server.stop_force()?
        },
    }

    Ok(time::sleep(Duration::from_secs(1)).await)
}

pub fn run_cli() -> Result<(), anyhow::Error> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    let cli = AppCli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    match cli.command {
        Some(SubCommands::Start(start_server_args)) => {
            runtime.block_on(start_server(start_server_args))
        }
        _ => Err(anyhow::Error::msg("not starting server")),
    }
}
