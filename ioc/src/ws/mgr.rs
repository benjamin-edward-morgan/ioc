use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::ws::message::*;
use crate::ws::state::*;

pub struct WsManager {
    pub handle: JoinHandle<()>,
    pub ws_sender: mpsc::Sender<WebSocket>,
}

impl WsManager {
    pub fn new(state_cmd_tx: mpsc::Sender<WsStateCmd>) -> WsManager {
        let (ws_sender, mut ws_receiver) = mpsc::channel::<WebSocket>(16);

        //task to connect new websockets
        let handle = tokio::spawn(async move {
            while let Some(socket) = ws_receiver.recv().await {
                info!("connecting websocket!");

                //get a state subscription
                let (subs_callback_tx, subs_callback_rx) = oneshot::channel();
                let subs_msg = WsStateCmd::SubscribeAll {
                    subs_callback: subs_callback_tx,
                };
                state_cmd_tx.send(subs_msg).await.unwrap();

                //websocket message sender and receiver
                let (mut ws_tx, mut ws_rx) = socket.split();

                //wait for the subscription callback
                let mut subs = subs_callback_rx.await.unwrap();

                //send initial state
                let initial_message = WsInitialMessage::from_state(subs.start);
                let json = serde_json::to_string(&initial_message).unwrap();
                if ws_tx.send(Message::Text(json)).await.is_ok() {
                    //send task transmits any state changes
                    let send_task = tokio::spawn(async move {
                        loop {
                            match subs.receiver.recv().await {
                                Ok(state) => {
                                    let update_message = WsUpdateMessage::from_state(state);
                                    let json = serde_json::to_string(&update_message).unwrap();
                                    if ws_tx.send(Message::Text(json)).await.is_err() {
                                        warn!("websocket send task failed!");
                                        break;
                                    }
                                }
                                Err(err) => {
                                    error!("ws mgr task shutting down bc of error! {}", err);
                                    break;
                                }
                            }
                        }
                    });

                    //receive task parses messages and sends update commands
                    let ws_state_cmd_tx = state_cmd_tx.clone();
                    tokio::spawn(async move {
                        while let Some(Ok(msg)) = ws_rx.next().await {
                            match msg {
                                Message::Text(t) => {
                                    match serde_json::from_str::<HashMap<String, WsStateUpdate>>(&t)
                                    {
                                        Ok(state) => {
                                            let update_msg = WsStateCmd::SetInputs {
                                                state: state
                                                    .iter()
                                                    .map(|(k, v)| {
                                                        (k.to_string(), (*v).clone().into_state())
                                                    })
                                                    .collect(),
                                            };
                                            if let Err(err) = ws_state_cmd_tx.send(update_msg).await
                                            {
                                                error!("ws send error: {:?}", err);
                                            }
                                        }
                                        e @ Err(..) => {
                                            warn!("got invalid json: {:?}", e);
                                        }
                                    }
                                }
                                m => {
                                    warn!("unexpected message type: {:?}", m);
                                }
                            }
                        }

                        send_task.abort();

                        info!("closed websocket!")
                    });
                }
            }
        });

        WsManager { handle, ws_sender }
    }
}
