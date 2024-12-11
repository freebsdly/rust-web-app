
use std::sync::{Arc, OnceLock};
use crate::api::{ApiService, ApiServiceArgs};
use serde::Deserialize;
use std::fmt::{Debug};
use anyhow::anyhow;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use crate::db::{DatabaseArgs, DbService};

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
    db_service: Arc<RwLock<DbService>>,
}

impl ServiceManager {
    pub async fn new(args: ServiceManagerArgs) -> Result<Self, anyhow::Error> {
        debug!("server args: {:?}", args.clone());
        let parent_token = CancellationToken::new();
        let db_service = DbService::new(args.database.clone()).await?;
        let db_service_arc = Arc::new(RwLock::new(db_service));
        let api_service = ApiService::new(parent_token.clone(), args.api.clone(),db_service_arc.clone())?;

        Ok(Self {
            args: OnceLock::from(args),
            parent_token,
            api_service: Arc::new(RwLock::new(api_service)),
            db_service: db_service_arc,
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