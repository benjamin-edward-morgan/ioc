use std::fmt::{Debug, Formatter};

use ioc_core::error::IocBuildError;

pub struct GpioError {
    pub message: String,
}

impl Debug for GpioError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str(&self.message)
    }
}

impl From<&str> for GpioError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

impl From<String> for GpioError {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}

impl From<rppal::gpio::Error> for GpioError {
    fn from(err: rppal::gpio::Error) -> Self {
        Self {
            message: format!("RpiGpioBuildError - Cause: {}", err),
        }
    }
}

impl From<GpioError> for IocBuildError {
    fn from(err: GpioError) -> Self {
        IocBuildError::from_string(err.message)
    }
}
