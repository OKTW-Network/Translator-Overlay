//! The process-wide ONNX Runtime environment.

use std::sync::LazyLock;

use ort::{environment::Environment, logging::LogLevel};

/// ONNX Runtime allows one environment per process, so OCR and the built-in translator share
/// this one. It logs errors only.
pub fn ort_environment() -> ort::Result<&'static Environment> {
    static ENVIRONMENT: LazyLock<ort::Result<Environment>> = LazyLock::new(|| {
        let environment = ort::init().build()?;
        environment.set_log_level(LogLevel::Error);
        Ok(environment)
    });
    ENVIRONMENT.as_ref().map_err(|e| ort::Error::new(e.to_string()))
}
