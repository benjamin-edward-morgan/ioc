//!A mod for the error types
use std::fmt::Debug;

///Common error type when using Configuration.
pub enum IocBuildError {
    Message(String),
    Messages(Vec<String>),
}

impl IocBuildError {
    pub fn from_string(msg: String) -> Self {
        IocBuildError::Message(msg)
    }
    pub fn from_errs(errs: Vec<IocBuildError>) -> Self {
        let mut messages = Vec::with_capacity(errs.len());
        for err in errs {
            match err {
                Self::Message(msg) => messages.push(msg),
                Self::Messages(mut msgs) => messages.append(&mut msgs),
            }
        }
        Self::Messages(messages)
    }
    pub fn message(msg: &str) -> Self {
        IocBuildError::Message(msg.to_string())
    }
    pub fn messages(msgs: &[String]) -> Self {
        IocBuildError::Messages(Vec::from(msgs).iter().map(|ptr| ptr.to_string()).collect())
    }
}

impl Debug for IocBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => f.write_fmt(format_args!("IocBuildError: {}", message)),
            Self::Messages(messages) => f.write_fmt(format_args!(
                "IocBuildError (multiple): \n{}",
                messages.join("\n")
            )),
        }
    }
}
