pub(crate) mod static_dir;
pub(crate) mod web_socket;
pub(crate) mod mjpeg_stream;

use crate::server::state::StateCmd;
use crate::EndpointConfig;
use axum::Router;
use static_dir::StaticDirEndpoint;
use web_socket::WebSocketEndpoint;
use mjpeg_stream::MjpegStreamEndpoint;

use tokio::sync::mpsc;

pub(crate) enum Endpoint {
    Static(StaticDirEndpoint),
    WebSocket(WebSocketEndpoint),
    MjpegStream(MjpegStreamEndpoint),
}

impl Endpoint {
    pub fn try_build(cmd_tx: &mpsc::Sender<StateCmd>, config: EndpointConfig) -> Self {
        match config {
            EndpointConfig::WebSocket{ inputs, outputs } => {
                let ws_endpoint = WebSocketEndpoint::new(cmd_tx, inputs, outputs);
                Endpoint::WebSocket(ws_endpoint)
            },
            EndpointConfig::Static{ directory } => {
                let static_endpoint = StaticDirEndpoint::new( directory );
                Endpoint::Static(static_endpoint)
            },
            EndpointConfig::Mjpeg { frames } => {
                let mjpeg_endpoint = MjpegStreamEndpoint{ frames };
                Endpoint::MjpegStream(mjpeg_endpoint)
            }
        }
    }

    pub fn apply(self, key: &str, router: Router) -> Router {
        match self {
            Self::WebSocket(endpoint) => endpoint.apply(key, router),
            Self::Static(endpoint) => endpoint.apply(key, router),
            Self::MjpegStream(endpoint) => endpoint.apply(key, router),
        }
    }

}
