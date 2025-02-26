use std::sync::Arc;
use axum::extract::State;
use axum_extra::extract::Query;
use serde::Deserialize;
use crate::api::{ApiResponse, ApiState};
use crate::api::jwt::Claims;
use crate::log::setup_logging_level;
use crate::user::UserInfo;

#[derive(Deserialize)]
pub struct BadgeQueryParams {
    badges: Option<Vec<String>>,
}

pub async fn query_badges(State(state): State<Arc<ApiState>>, params: Query<BadgeQueryParams>) -> ApiResponse<Vec<UserInfo>> {
    let service = state.user_service.read().await;
    let badges = params.badges.clone().unwrap_or_default();
    let result = service.query_user_infos(badges).await;
    ApiResponse::ok(result.ok())
}

pub async fn health() -> ApiResponse<String> {
    ApiResponse::ok(Some(String::from("healthy")))
}

pub async fn test(claims: Claims) -> ApiResponse<String> {
    ApiResponse::ok(Some(String::from("test")))
}

#[derive(Deserialize)]
pub struct LoggingLevelParams {
   level: Option<String>,
}

pub async fn logging(Query(loglevel): Query<LoggingLevelParams>) -> ApiResponse<String> {
    if let Some(level) = loglevel.level {
        let result = setup_logging_level(level).await;
        if result.is_err() {
            ApiResponse::err(-1, "failed".to_string())
        } else {
            ApiResponse::ok(Some(String::from("success")))
        }
    } else {
        ApiResponse::ok(Some(String::from("success")))
    }
}
