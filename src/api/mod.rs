mod jwt;
mod error;

use axum::error_handling::HandleErrorLayer;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{serve, BoxError, Json, Router};
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::GenericMetricLayer;
use axum_prometheus::Handle;
use axum_prometheus::PrometheusMetricLayerBuilder;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug};
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use crate::api::jwt::Claims;

#[derive(Serialize, Debug)]
pub struct ApiResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(code: i32, message: String, data: Option<T>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    pub fn ok(data: Option<T>) -> Self {
        Self::new(0, String::from("successful"), data)
    }

    pub fn err(code: i32, message: String) -> Self {
        Self::new(code, message, None)
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    Json<ApiResponse<T>>: IntoResponse,
{
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

pub async fn handler_404(method: Method, uri: Uri) -> (StatusCode, ApiResponse<String>) {
    (
        StatusCode::NOT_FOUND,
        ApiResponse::err(-1, format!("{} {} Not Found", method, uri)),
    )
}

pub async fn handle_error(
    // `Method` and `Uri` are extractors so they can be used here
    method: Method,
    uri: Uri,
    // the last argument must be the error itself
    err: BoxError,
) -> (StatusCode, ApiResponse<String>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        ApiResponse::new(
            -1,
            format!("{} {} failed", method, uri),
            Some(err.to_string()),
        ),
    )
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApiServiceArgs {
    #[serde(alias = "address")]
    pub address: String,
    pub port: u16,
    pub timeout: u64,
}

pub struct ApiService {
    args: ApiServiceArgs,
    tx: broadcast::Sender<()>,
}

impl ApiService {
    pub fn new(args: ApiServiceArgs) -> Self {
        let (tx, _) = broadcast::channel::<()>(1);
        Self { args, tx }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        // Create prometheus layer
        let (prometheus_layer, metric_handle) = Self::build_metrics();
        // Create a regular axum app.
        let app = Router::new()
            .route("/test", get(Self::test))
            .route("/health", get(Self::health))
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .fallback(handler_404)
            // request trace
            .layer(TraceLayer::new_for_http())
            // request timeout
            .layer(TimeoutLayer::new(Duration::from_secs(self.args.timeout)))
            // prometheus metric
            .layer(prometheus_layer)
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .timeout(Duration::from_secs(self.args.timeout)),
            );

        // Create a `TcpListener` using tokio.
        let addr = format!("{}:{}", self.args.address, self.args.port);
        let listener = TcpListener::bind(addr).await?;
        info!("listening on {}", listener.local_addr()?);
        // Run the server with graceful shutdown
        let mut rx = self.tx.subscribe();
        let graceful_timeout = self.args.timeout.clone();
        let _ = serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = rx.recv().await;
                // wait for timeout to make request finished
                info!("wait {} secs for graceful shutdown", graceful_timeout);
                sleep(Duration::from_secs(graceful_timeout)).await;
            })
            .await;
        Ok(())
    }

    fn build_metrics() -> (
        GenericMetricLayer<'static, PrometheusHandle, Handle>,
        PrometheusHandle,
    ) {
        PrometheusMetricLayerBuilder::new()
            .with_ignore_patterns(&["/metrics"])
            // .with_group_patterns_as("/foo", &["/foo/:bar", "/foo/:bar/:baz"])
            // .with_group_patterns_as("/bar", &["/auth/*path"])
            .with_default_metrics()
            .build_pair()
    }

    fn stop(&self) -> Result<(), anyhow::Error> {
        self.tx.send(()).expect("send stop sig failed");
        info!("Stopping ApiService");
        Ok(())
    }

    async fn health() -> ApiResponse<String> {
        // sleep(Duration::from_secs(1)).await;
        ApiResponse::ok(None::<String>)
    }

    async fn test(claims: Claims) -> ApiResponse<String> {
        ApiResponse::ok(Some(String::from("test")))
    }
}
