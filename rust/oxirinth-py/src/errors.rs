use oxirinth_engine::error::EngineError;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

pyo3::create_exception!(
    oxirinth._core,
    OxirinthError,
    PyException,
    "Base exception for all Oxirinth SDK errors."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthConfigError,
    OxirinthError,
    "Invalid client, cache, or query configuration."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthNetworkError,
    OxirinthError,
    "HTTP transport, TLS, or timeout error."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthAPIError,
    OxirinthError,
    "Non-success HTTP response returned by Modrinth API."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthNotFoundError,
    OxirinthAPIError,
    "Requested Modrinth resource was not found (HTTP 404)."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthAuthError,
    OxirinthAPIError,
    "Authentication or authorization failure (HTTP 401/403)."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthRateLimitError,
    OxirinthAPIError,
    "Modrinth API rate limit exceeded and wait limit exhausted (HTTP 429)."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthTypeMismatchError,
    OxirinthError,
    "Retrieved project does not belong to the requested Modrinth domain."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthCacheError,
    OxirinthError,
    "Local redb/zstd or memory cache error."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthUnboundModelError,
    OxirinthError,
    "Helper method requiring a client was invoked on an unbound model."
);
pyo3::create_exception!(
    oxirinth._core,
    OxirinthInternalError,
    OxirinthError,
    "Unexpected internal Rust panic or invariant violation."
);

/// Register all Oxirinth exception classes and default structured attributes on `oxirinth._core`.
pub fn register_exceptions(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let api_cls = py.get_type::<OxirinthAPIError>();
    api_cls.setattr("status_code", 0u16)?;
    api_cls.setattr("error_code", py.None())?;
    api_cls.setattr("description", "")?;
    api_cls.setattr("url", "")?;

    let rate_cls = py.get_type::<OxirinthRateLimitError>();
    rate_cls.setattr("retry_after_secs", 0u64)?;

    let mismatch_cls = py.get_type::<OxirinthTypeMismatchError>();
    mismatch_cls.setattr("expected", "")?;
    mismatch_cls.setattr("actual", PyTuple::empty(py))?;
    mismatch_cls.setattr("identifier", "")?;

    m.add("OxirinthError", py.get_type::<OxirinthError>())?;
    m.add("OxirinthConfigError", py.get_type::<OxirinthConfigError>())?;
    m.add(
        "OxirinthNetworkError",
        py.get_type::<OxirinthNetworkError>(),
    )?;
    m.add("OxirinthAPIError", api_cls)?;
    m.add(
        "OxirinthNotFoundError",
        py.get_type::<OxirinthNotFoundError>(),
    )?;
    m.add("OxirinthAuthError", py.get_type::<OxirinthAuthError>())?;
    m.add("OxirinthRateLimitError", rate_cls)?;
    m.add("OxirinthTypeMismatchError", mismatch_cls)?;
    m.add("OxirinthCacheError", py.get_type::<OxirinthCacheError>())?;
    m.add(
        "OxirinthUnboundModelError",
        py.get_type::<OxirinthUnboundModelError>(),
    )?;
    m.add(
        "OxirinthInternalError",
        py.get_type::<OxirinthInternalError>(),
    )?;
    Ok(())
}

/// Convert an [`EngineError`] into its corresponding Python exception with structured attributes.
pub fn engine_error_to_py(err: EngineError) -> PyErr {
    let message = err.to_string();
    Python::attach(|py| match err {
        EngineError::Config(_) => OxirinthConfigError::new_err(message),
        EngineError::Network { .. } => OxirinthNetworkError::new_err(message),
        EngineError::Api {
            status,
            error_code,
            description,
            url,
        } => {
            let py_err = OxirinthAPIError::new_err(message);
            attach_api_attrs(py, &py_err, status, error_code, &description, &url);
            py_err
        }
        EngineError::NotFound {
            description, url, ..
        } => {
            let py_err = OxirinthNotFoundError::new_err(message);
            attach_api_attrs(
                py,
                &py_err,
                404,
                Some("not_found".to_owned()),
                &description,
                &url,
            );
            py_err
        }
        EngineError::Auth {
            status,
            description,
            url,
        } => {
            let py_err = OxirinthAuthError::new_err(message);
            attach_api_attrs(
                py,
                &py_err,
                status,
                Some("unauthorized".to_owned()),
                &description,
                &url,
            );
            py_err
        }
        EngineError::RateLimited {
            retry_after_secs,
            url,
        } => {
            let py_err = OxirinthRateLimitError::new_err(message);
            attach_api_attrs(
                py,
                &py_err,
                429,
                Some("rate_limited".to_owned()),
                &format!("Rate limit exceeded; retry after {retry_after_secs}s"),
                &url,
            );
            let _ = py_err
                .value(py)
                .setattr("retry_after_secs", retry_after_secs);
            py_err
        }
        EngineError::TypeMismatch {
            expected,
            actual,
            identifier,
        } => {
            let py_err = OxirinthTypeMismatchError::new_err(message);
            let exc = py_err.value(py);
            let _ = exc.setattr("expected", expected);
            if let Ok(actual_tuple) = PyTuple::new(py, &actual) {
                let _ = exc.setattr("actual", actual_tuple);
            }
            let _ = exc.setattr("identifier", identifier);
            py_err
        }
        EngineError::Cache(_) => OxirinthCacheError::new_err(message),
        EngineError::Serialization(_) | EngineError::Internal(_) => {
            OxirinthInternalError::new_err(message)
        }
        _ => OxirinthInternalError::new_err(message),
    })
}

fn attach_api_attrs(
    py: Python<'_>,
    py_err: &PyErr,
    status: u16,
    error_code: Option<String>,
    description: &str,
    url: &str,
) {
    let exc = py_err.value(py);
    let _ = exc.setattr("status_code", status);
    let _ = exc.setattr("error_code", error_code);
    let _ = exc.setattr("description", description);
    let _ = exc.setattr("url", url);
}
