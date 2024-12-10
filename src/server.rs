
use std::sync::{Arc, OnceLock};
use crate::api::{ApiService, ApiServiceArgs};
use serde::Deserialize;
use std::fmt::{Debug};
use anyhow::anyhow;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
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
    parent_token: CancellationToken,
    api_service: Arc<RwLock<ApiService>>,
}

impl ServiceManager {
    pub fn new(args: ServiceManagerArgs) -> Result<Self, anyhow::Error> {
        debug!("server args: {:?}", args.clone());
        let parent_token = CancellationToken::new();
        let api_service = ApiService::new(parent_token.clone(), args.api.clone())?;
        Ok(Self {
            args: OnceLock::from(args),
            parent_token,
            api_service: Arc::new(RwLock::new(api_service)),
        })
    }

    pub fn start(&self) -> Result<(), anyhow::Error> {
        self.start_api_service()?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), anyhow::Error> {
        info!("Stopping ServerManager gracefully");
        self.stop_api_service()?;
        Ok(())
    }

    pub fn stop_force(&self) -> Result<(), anyhow::Error> {
        info!("Stopping ServerManager force");
        self.parent_token.cancel();
        Ok(())
    }

    fn start_api_service(&self) -> Result<(), anyhow::Error> {
       let api_service = self.api_service.clone();
        let guard = api_service.try_write();
        match guard {
            Ok(guard) => {
                guard.start()
            }
            Err(err) => {
                Err(anyhow!(err))
            }
        }
    }

    fn stop_api_service(&self) -> Result<(), anyhow::Error> {
        let api_service = self.api_service.clone();
        let guard = api_service.try_write();
        match guard {
            Ok(service) => {
                service.stop()
            }
            Err(err) => {
                Err(anyhow!(err))
            }
        }
    }
}