use axum::Router;
use tower_http::services::ServeDir;

pub(crate) struct StaticDirEndpoint {
    directory: String,
}

impl StaticDirEndpoint {
    pub fn new(directory: &str) -> Self {
        Self {
            directory: directory.to_string(),
        }
    }

    pub fn apply(self, _key: &str, router: Router) -> Router {
        router
            .fallback_service(ServeDir::new(self.directory).append_index_html_on_directories(true))
    }
}
