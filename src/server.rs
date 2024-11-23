use crate::api::{ApiServer, ApiServerArgs};
use serde::Deserialize;
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::{select, signal};
use tokio::time::sleep;
use tracing::{debug, info};

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseArgs {
    #[serde(alias = "type")]
    db_type: String,
    #[serde(alias = "host")]
    db_host: String,
    #[serde(alias = "port")]
    db_port: u16,
    #[serde(alias = "name")]
    db_name: String,
    username: String,
    password: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerArgs {
    #[serde(alias = "common")]
    api: ApiServerArgs,
    #[serde(alias = "database")]
    pub database: DatabaseArgs,
}

pub struct Server {
    args: ServerArgs,
    api_server: ApiServer,
    runtime: Runtime,
}

impl Server {
    pub fn new(args: ServerArgs) -> Result<Self, anyhow::Error> {
        debug!("Creating server args: {:?}", args.clone());
        let api_server = ApiServer::new(args.api.clone());
        let runtime = Runtime::new()?;
        Ok(Server {
            args,
            api_server,
            runtime,
        })
    }

    pub fn start(&self) -> Result<(), anyhow::Error> {
        info!("Starting Server");
        let api_cfg = self.args.api.clone();

        let result = self.runtime.spawn( async {
            info!("Starting API server");
            let api_server = ApiServer::new(api_cfg);
            api_server.start().await.expect("API server error");
        });

        // self.runtime.block_on(async {
        //     self.runtime.spawn(async move {
        //         info!("Starting MQ server");
        //         sleep(Duration::from_secs(10)).await;
        //         info!("MQ server started Successfuly");
        //     });
        // });
        info!("Server started successfully");
        Ok(())
    }

    pub fn stop(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}