use std::fmt::{Debug, Formatter};

pub struct BuildError {
    pub message: String,
}

impl Debug for BuildError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        fmt.write_str(&self.message)
    }
}

impl From<&str> for BuildError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

impl From<String> for BuildError {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}

impl From<rppal::gpio::Error> for BuildError {
    fn from(err: rppal::gpio::Error) -> Self {
        Self {
            message: format!("RpiGpioBuildError - Cause: {}", err),
        }
    }
}
