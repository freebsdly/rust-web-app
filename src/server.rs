
use std::sync::{OnceLock};
use crate::api::{ApiService, ApiServiceArgs};
use serde::Deserialize;
use std::fmt::{Debug};
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
    api_service_handle: Option<JoinHandle<()>>,
    mq_service_handle: Option<JoinHandle<()>>,
    db_service_handle: Option<JoinHandle<()>>,
}

impl ServiceManager {
    pub fn new(args: ServiceManagerArgs) -> Result<Self, anyhow::Error> {
        debug!("server args: {:?}", args.clone());
        let runtime = Runtime::new()?;
        Ok(Self {
            args: OnceLock::from(args),
            runtime,
            api_service_handle: None,
            mq_service_handle: None,
            db_service_handle: None,
        })
    }

    pub fn start(&mut self) -> Result<(), anyhow::Error> {
        self.start_db_service()?;
        self.start_mq_service()?;
        self.start_api_service()?;
        Ok(())
    }

    fn start_api_service(&mut self) -> Result<(), anyhow::Error> {
        let api_cfg = self.args.take().unwrap().api.clone();
        let api_service_handle = self.runtime.spawn( async {
            info!("Starting API service");
            let mut api_server = ApiService::new(api_cfg);
            api_server.start().await.expect("API service error");
        });
        self.api_service_handle = Some(api_service_handle);
        Ok(())
    }

    fn start_mq_service(&mut self) -> Result<(), anyhow::Error> {
        let mq_service_handle = self.runtime.spawn( async {
            info!("Starting MQ service");
        });
        self.mq_service_handle = Some(mq_service_handle);
        Ok(())
    }

    fn start_db_service(&mut self) -> Result<(), anyhow::Error> {
        let db_service_handle = self.runtime.spawn( async {
            info!("Starting DB service");
        });
        self.db_service_handle = Some(db_service_handle);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), anyhow::Error> {
        info!("Stopping ServerManager gracefully");
        Ok(())
    }

    pub fn stop_force(&mut self) -> Result<(), anyhow::Error> {
        info!("Force to stop ServerManager");
        if let Some(handle) = self.api_service_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.mq_service_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.db_service_handle.take() {
            handle.abort();
        }
        info!("Server stopped successfully");
        Ok(())
    }
}