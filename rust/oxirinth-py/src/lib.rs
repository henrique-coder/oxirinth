use pyo3::prelude::*;

mod bridge;
mod errors;
mod runtime;

use self::bridge::{AsyncCore, PyCacheStats, SyncCore};

/// Return the compiled `oxirinth-engine` semantic version.
#[pyfunction]
fn engine_version() -> &'static str {
    oxirinth_engine::ENGINE_VERSION
}

#[pymodule(gil_used = false)]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    oxirinth_engine::init_crypto_provider();
    runtime::init_runtime()?;

    m.add("__version__", oxirinth_engine::ENGINE_VERSION)?;
    m.add(
        "DEFAULT_USER_AGENT",
        oxirinth_engine::config::default_user_agent(),
    )?;
    m.add_function(wrap_pyfunction!(engine_version, m)?)?;
    errors::register_exceptions(m.py(), m)?;
    m.add_class::<PyCacheStats>()?;
    m.add_class::<SyncCore>()?;
    m.add_class::<AsyncCore>()?;
    Ok(())
}
