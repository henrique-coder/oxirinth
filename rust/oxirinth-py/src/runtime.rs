use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;

use pyo3::PyResult;
use tokio::runtime::{Builder, Runtime};

use crate::errors::OxirinthInternalError;

const ISSUE_URL: &str = "https://github.com/henrique-coder/oxirinth/issues";

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Return the shared multi-threaded Tokio runtime powering both sync and async FFI bridges.
pub fn get_runtime() -> PyResult<&'static Runtime> {
    if let Some(rt) = TOKIO_RUNTIME.get() {
        return Ok(rt);
    }

    let built = Builder::new_multi_thread()
        .enable_all()
        .thread_name("oxirinth-worker")
        .build()
        .map_err(|err| {
            OxirinthInternalError::new_err(format!(
                "failed to start Oxirinth Tokio runtime: {err}. Please report this at {ISSUE_URL}"
            ))
        })?;

    Ok(TOKIO_RUNTIME.get_or_init(|| built))
}

/// Initialize the global Tokio runtime and bind it to `pyo3-async-runtimes`.
pub fn init_runtime() -> PyResult<()> {
    let rt = get_runtime()?;
    let _ = pyo3_async_runtimes::tokio::init_with_runtime(rt);
    Ok(())
}

/// Execute `f` behind a hard panic boundary, converting any Rust panic into `OxirinthInternalError`.
pub fn catch_unwind_py<F, T>(f: F) -> PyResult<T>
where
    F: FnOnce() -> PyResult<T>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(res) => res,
        Err(payload) => {
            let detail = extract_panic_message(&payload);
            Err(OxirinthInternalError::new_err(format!(
                "unexpected panic in oxirinth._core: {detail}. Please report this bug at {ISSUE_URL}"
            )))
        }
    }
}

fn extract_panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return msg;
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.as_str();
    }
    "unknown panic payload"
}
