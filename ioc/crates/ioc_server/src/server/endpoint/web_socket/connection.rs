use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use super::message::{WsInitialMessage, WsStateUpdate, WsUpdateMessage};
use crate::server::state::{StateCmd, Subscription};

pub(crate) struct WebSocketConnection {
    _handle: JoinHandle<()>,
}

impl WebSocketConnection {
    pub async fn new(
        state_cmd_tx: &mpsc::Sender<StateCmd>,
        web_socket: WebSocket,
        subscription: Subscription,
    ) -> Self {
        //websocket message sender and receiver
        let (mut ws_tx, mut ws_rx) = web_socket.split();

        //global server state update receiver
        let mut update_rx = subscription.update_rx;

        //global server state update sender
        let state_cmd_tx = state_cmd_tx.clone();

        //send intitial message
        let initial_message: WsInitialMessage = subscription.start.into();
        let json = serde_json::to_string(&initial_message).unwrap();
        match ws_tx.send(Message::Text(json)).await {
            Ok(_) => {
                debug!("sent intial ws message. starting send task ... ");
                let send_task = tokio::spawn(async move {
                    while let Ok(update) = update_rx.recv().await {
                        let update_msg: WsUpdateMessage = update.into();
                        let json = serde_json::to_string(&update_msg).unwrap();
                        ws_tx.send(Message::Text(json)).await.unwrap();
                    }
                    info!("websocket send task is done!");
                });

                let handle = tokio::spawn(async move {
                    while let Some(Ok(message)) = ws_rx.next().await {
                        match message {
                            Message::Text(text) => {
                                match serde_json::from_str::<HashMap<String, WsStateUpdate>>(&text)
                                {
                                    Ok(updates) => {
                                        let state_cmd = StateCmd::Update(updates.into());
                                        state_cmd_tx.send(state_cmd).await.unwrap();
                                    }
                                    Err(err) => {
                                        warn!("could not parse {}", err);
                                    }
                                }
                            }
                            Message::Close(frame_opt) => {
                                debug!(
                                    "closing websocket because we recieved close frame: {:?}",
                                    frame_opt
                                );
                            }
                            message => {
                                warn!("unexpected message type! {:?}", message)
                            }
                        }
                    }
                    send_task.abort();
                    debug!("websocket is closing");
                });

                WebSocketConnection { _handle: handle }
            }
            Err(err) => {
                panic!("error sending initial ws message! {}", err);
            }
        }
    }
}
