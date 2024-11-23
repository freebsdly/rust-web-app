use crate::server::ServerArgs;
use axum::error_handling::HandleErrorLayer;
use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{serve, BoxError, Json, Router};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::sync::oneshot::{Receiver, Sender};
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Serialize, Debug)]
pub struct ApiResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(code: i32, message: String, data: Option<T>) -> Self {
        Self { code, message, data }
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
    (StatusCode::NOT_FOUND, ApiResponse::err(-1, format!("{} {} Not Found", method, uri)))
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("internal error")]
    InternalError,
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
        ApiResponse::new(-1, format!("{} {} failed", method, uri), Some(err.to_string()))
    )
}


#[derive(Deserialize, Debug, Clone)]
pub struct ApiServerArgs {
    #[serde(alias = "address")]
    pub address: String,
    pub port: u16,
    pub timeout: u64,
}

pub struct ApiServer {
    args: ApiServerArgs,
    sender: Sender<()>,
    receiver: Receiver<()>,
}

impl ApiServer {
    pub fn new(args: ApiServerArgs) -> Self {
        let (tx, rx) = oneshot::channel::<()>();
        Self { args, sender: tx, receiver: rx }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        // Create a regular axum app.
        let app = Router::new()
            .route("/health", get(Self::health))
            .fallback(handler_404)
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            // .layer(TimeoutLayer::new(Duration::from_secs(self.args.timeout))
            .layer((
                TraceLayer::new_for_http(),
                // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
                // requests don't hang forever.
                TimeoutLayer::new(Duration::from_secs(self.args.timeout)),
            ))
            .layer(ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .timeout(Duration::from_secs(self.args.timeout))
            );

        // Create a `TcpListener` using tokio.
        let addr = format!("{}:{}", self.args.address, self.args.port);
        let listener = TcpListener::bind(addr).await?;
        info!("listening on {}", listener.local_addr()?);
        // Run the server with graceful shutdown
        let _ = serve(listener, app)
            // .with_graceful_shutdown(async { })
            .await;
        Ok(())
    }

    fn stop(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn health() -> ApiResponse<String> {
        ApiResponse::ok(None::<String>)
    }
}