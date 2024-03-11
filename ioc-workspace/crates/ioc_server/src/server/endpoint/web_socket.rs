pub(crate) mod manager;
pub(crate) mod connection;
pub(crate) mod message;

use crate::server::state::StateCmd;
use axum::Router;
use manager::WebSocketManager;

use axum::routing::get;
use axum::response::IntoResponse;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::State;
use tokio::sync::mpsc;

pub(crate) struct WebSocketEndpoint {
    ws_mgr: WebSocketManager
}

impl WebSocketEndpoint {
    pub fn new(cmd_tx: &mpsc::Sender<StateCmd>, inputs: &[String], outputs: &[String]) -> Self {
        let ws_mgr = WebSocketManager::new(cmd_tx, inputs, outputs);

        Self{
            ws_mgr
        }
    }

    // pub fn method_router(&self) -> MethodRouter {
    //     get(handle_ws_upgrade).with_state(self.ws_mgr.websocket_tx.clone())
    // }

    pub fn apply(self, key: &str, router: Router) -> Router {
        router.route(key, 
            get(handle_ws_upgrade).with_state(self.ws_mgr.websocket_tx.clone())
        )
    }
}

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(ws_tx): State<mpsc::Sender<WebSocket>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move { 
        ws_tx.send(socket).await.unwrap();
    })
}