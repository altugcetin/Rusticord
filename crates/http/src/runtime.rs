use std::sync::OnceLock;

use tokio::runtime::{Handle, Runtime};

use crate::error::HttpError;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn runtime_handle() -> Result<Handle, HttpError> {
    if let Ok(handle) = Handle::try_current() {
        return Ok(handle);
    }
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime.handle().clone());
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("rusticord-http")
        .build()
        .map_err(|_| HttpError::Transport)?;
    let _ = RUNTIME.set(runtime);
    RUNTIME
        .get()
        .map(|runtime| runtime.handle().clone())
        .ok_or(HttpError::Transport)
}

#[cfg(test)]
mod tests {
    use super::runtime_handle;

    #[tokio::test]
    async fn prefers_the_current_tokio_runtime() {
        assert!(runtime_handle().is_ok());
    }
}
