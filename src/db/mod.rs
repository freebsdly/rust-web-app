use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

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
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
}

impl DatabaseArgs {
    pub fn dsn(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/{}",
            self.db_type, self.username, self.password, self.db_host, self.db_port, self.db_name
        )
    }
}

pub struct DbService {
    args: DatabaseArgs,
    pub pool: PgPool,
}

impl DbService {
    pub async fn new(args: DatabaseArgs) -> Result<Self, anyhow::Error> {
        info!("Connecting to database: {:?}", args.dsn());
        let pool = PgPoolOptions::new()
            .max_connections(args.max_connections.unwrap_or(20))
            .min_connections(args.min_connections.unwrap_or(5))
            .connect(&args.dsn())
            .await?;
        Ok(DbService { args, pool })
    }

    pub async fn close(&self) -> Result<(), anyhow::Error> {
        info!("Closing database connection");
        Ok(self.pool.close().await)
    }
}
