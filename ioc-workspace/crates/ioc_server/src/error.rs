#[derive(Debug)]
pub struct ServerBuildError {
    pub message: String,
}

impl ServerBuildError {
    pub(crate) fn new(s: String) -> Self {
        Self { message: s }
    }
}
