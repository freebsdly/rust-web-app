use crate::db::DbService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize, Serialize, sqlx::FromRow, Debug, PartialEq, Eq)]
pub struct UserInfo {
    badge: Option<String>,
    name: Option<String>,
}

pub struct UserService {
    db_service: Arc<RwLock<DbService>>,
}

impl UserService {
    pub fn new(db_service: Arc<RwLock<DbService>>) -> UserService {
        UserService { db_service }
    }

    pub async fn query_user_infos(
        &self,
        badges: Vec<String>,
    ) -> Result<Vec<UserInfo>, anyhow::Error> {
        let x = 1 ..= badges.len();
        let placer = x.map(|num| { format!("${}", num)})
            .reduce(|acc, e| format!("{},{}", acc, e)).unwrap();
        let sql = format!("select * from tb_cmdb_users where badge in ({})", placer);
        let db_service = self.db_service.clone();
        let service = db_service.read().await;
        let mut query = sqlx::query_as(sql.as_str());
        for badge in badges {
            query = query.bind(badge)
        }

        let result = query.fetch_all(&service.pool).await?;
        Ok(result)
    }
}
