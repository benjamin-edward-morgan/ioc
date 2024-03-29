use super::connection::WebSocketConnection;
use crate::server::state::StateCmd;

use axum::extract::ws::WebSocket;
use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

pub(crate) struct WebSocketManager {
    pub websocket_tx: mpsc::Sender<WebSocket>,
}

impl WebSocketManager {
    pub fn new(cmd_tx: &mpsc::Sender<StateCmd>, inputs: &[String], outputs: &[String]) -> Self {
        let inputs: HashSet<String> = inputs.iter().map(|s| s.to_string()).collect();
        let outputs: HashSet<String> = outputs.iter().map(|s| s.to_string()).collect();
        let (websocket_tx, mut websocket_rx) = mpsc::channel(10);

        let task_state_cmd_tx = cmd_tx.clone();
        tokio::spawn(async move {
            while let Some(websocket) = websocket_rx.recv().await {
                debug!("conneting new websocket");

                //get a state subscription
                let (callback_tx, callback_rx) = oneshot::channel();
                let subs_cmd = StateCmd::Subscribe {
                    callback: callback_tx,
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                };
                let subs_option = match task_state_cmd_tx.send(subs_cmd).await {
                    Err(send_err) => {
                        error!(
                            "error sending subscription command in websocket manager! {}",
                            send_err
                        );
                        None
                    }
                    Ok(_) => match callback_rx.await {
                        Ok(subs) => Some(subs),
                        Err(recv_err) => {
                            error!(
                                "error receiving subscription in websocket manager! {}",
                                recv_err
                            );
                            None
                        }
                    },
                };

                if let Some(subscription) = subs_option {
                    let _connection =
                        WebSocketConnection::new(&task_state_cmd_tx, websocket, subscription).await;
                }
            }
            debug!("websocket manager is done!");
        });

        Self { websocket_tx }
    }
}
