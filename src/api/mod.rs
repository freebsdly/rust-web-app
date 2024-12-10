mod error;
mod jwt;

use crate::api::jwt::Claims;
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
use std::fmt::Debug;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::select;
use tokio_util::sync::CancellationToken;
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
    cancel_token: CancellationToken,
}

impl ApiService {
    pub fn new(token: CancellationToken, args: ApiServiceArgs) -> Result<Self, anyhow::Error> {
        Ok(Self {
            cancel_token: token,
            args,
        })
    }

    fn protected_routes() -> Router {
        Router::new().route("/test", get(Self::test))
    }

    fn opened_routes() -> Router {
        Router::new().route("/health", get(Self::health))
    }

    pub fn start(&self) -> Result<(), anyhow::Error> {
        info!("starting api service");
        let token = self.cancel_token.clone();
        let args = self.args.clone();
        let addr = format!("{}:{}", args.address, args.port);
        info!("listening on {}", addr);
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        tokio::spawn(Self::start_app(token, listener, args));
        Ok(())
    }

    async fn start_app(
        token: CancellationToken,
        listener: std::net::TcpListener,
        args: ApiServiceArgs,
    ) -> Result<(), anyhow::Error> {
        let (prometheus_layer, metric_handle) = Self::build_metrics();
        // Create a regular axum app.
        let app = Router::new()
            .nest("/protected", Self::protected_routes())
            .nest("/opened", Self::opened_routes())
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .fallback(handler_404)
            // request trace
            .layer(TraceLayer::new_for_http())
            // request timeout
            .layer(TimeoutLayer::new(Duration::from_secs(args.timeout)))
            // prometheus metric
            .layer(prometheus_layer)
            .layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(handle_error))
                    .timeout(Duration::from_secs(args.timeout)),
            );

        let tcp_listener = TcpListener::from_std(listener)?;
        // Run the server with graceful shutdown
        let _ = serve(tcp_listener, app)
            .with_graceful_shutdown(async move {
                select! {
                    _ = token.cancelled() => {
                        info!("received shutdown api service signal");
                    },
                }
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

    pub fn stop(&self) -> Result<(), anyhow::Error> {
        info!("Stopping ApiService");
        self.cancel_token.cancel();
        Ok(())
    }

    async fn health() -> ApiResponse<String> {
        ApiResponse::ok(None::<String>)
    }

    async fn test(claims: Claims) -> ApiResponse<String> {
        ApiResponse::ok(Some(String::from("test")))
    }
}
