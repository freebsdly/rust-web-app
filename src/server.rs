
use std::sync::{OnceLock};
use crate::api::{ApiService, ApiServiceArgs};
use serde::Deserialize;
use std::fmt::{Debug};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tracing::{debug, info};

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseArgs {
    #[serde(alias = "type")]
    pub db_type: String,
    #[serde(alias = "host")]
    pub db_host: String,
    #[serde(alias = "port")]
    pub db_port: u16,
    #[serde(alias = "name")]
    pub db_name: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServiceManagerArgs {
    #[serde(alias = "common")]
    pub(crate) api: ApiServiceArgs,
    #[serde(alias = "database")]
    pub database: DatabaseArgs,
}

pub struct ServiceManager {
    args: OnceLock<ServiceManagerArgs>,
    runtime: Runtime,
    api_server_handle: Option<JoinHandle<()>>
}

impl ServiceManager {
    pub fn new(args: ServiceManagerArgs) -> Result<Self, anyhow::Error> {
        debug!("Creating server args: {:?}", args.clone());
        let runtime = Runtime::new()?;
        Ok(Self {
            args: OnceLock::from(args),
            runtime,
            api_server_handle: None,
        })
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        self.start_api_service()?;
        Ok(())
    }

    fn start_api_service(&mut self) -> Result<(), anyhow::Error> {
        // TODO: 使用channel启动stop线程
        let api_cfg = self.args.take().unwrap().api;
        let api_server_handle = self.runtime.spawn( async {
            info!("Starting API service");
            let mut api_server = ApiService::new(api_cfg);
            api_server.start().await.expect("API service error");
        });
        self.api_server_handle = Some(api_server_handle);
        Ok(())
    }

    fn start_mq_service(&mut self) -> Result<(), anyhow::Error> {
        info!("Starting MQ service");
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), anyhow::Error> {
        info!("Stopping Server");
        if let Some(handle) = self.api_server_handle.take() {
            handle.abort()
        }
        std::thread::sleep(Duration::from_secs(2));
        info!("Server stopped successfully");
        Ok(())
    }
}