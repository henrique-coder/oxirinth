use pyo3::prelude::*;

/// Return the compiled `oxirinth-engine` semantic version.
#[pyfunction]
fn engine_version() -> &'static str {
    oxirinth_engine::ENGINE_VERSION
}

#[pymodule(gil_used = false)]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    oxirinth_engine::init_crypto_provider();
    m.add("__version__", oxirinth_engine::ENGINE_VERSION)?;
    m.add_function(wrap_pyfunction!(engine_version, m)?)?;
    Ok(())
}
