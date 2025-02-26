use std::sync::{LazyLock};
use tokio::sync::Mutex;
use tracing_subscriber::{fmt, EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::{Handle, Layer};
use tracing_subscriber::util::SubscriberInitExt;

struct GlobalLogReloadHandle(Option<Handle<EnvFilter, Registry>>);

impl GlobalLogReloadHandle {
    fn new() -> Self {
        GlobalLogReloadHandle(None)
    }

    fn set(&mut self, handle: Handle<EnvFilter, Registry>) {
        self.0 = Some(handle);
    }

    fn get(&self) -> Option<Handle<EnvFilter, Registry>> {
        self.0.clone()
    }
}

static GLOBAL_LOG_RELOAD_HANDLE: LazyLock<Mutex<GlobalLogReloadHandle>> = LazyLock::new(|| {
    Mutex::new(GlobalLogReloadHandle::new())
});

pub async fn init_logging() -> Result<(), anyhow::Error> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))?;
    let (filter, reload_handle) = Layer::new(env_filter);
    let mut guard = GLOBAL_LOG_RELOAD_HANDLE.lock().await;
    guard.set(reload_handle);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::Layer::default())
        .init();
    Ok(())
}

pub async fn setup_logging_level(level: String) -> Result<(), anyhow::Error> {
    let env_filter = EnvFilter::try_new(level)?;
    let guard = GLOBAL_LOG_RELOAD_HANDLE.lock().await;
    if let Some(reload_handle) = guard.get() {
        reload_handle.modify(|filter| *filter = env_filter)?;
    }
    Ok(())
}