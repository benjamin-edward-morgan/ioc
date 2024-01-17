use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{collections::HashMap, net::SocketAddr};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, TraceLayer},
};

pub mod input;
mod message;
mod mgr;
pub mod output;
mod state;

use mgr::WsManager;
use state::WsState;

use {input::WsInput, output::WsOutput, state::WsStateCmd};

#[derive(Deserialize, Debug)]
pub struct WsInputFloatConfig {
    start: f64,
    min: f64,
    max: f64,
    step: f64,
}

#[derive(Deserialize, Debug)]
pub struct WsInputBoolConfig {
    start: bool,
}

#[derive(Deserialize, Debug)]
pub struct WsInputStringConfig {
    start: String,
    max_length: usize,
}

#[derive(Debug)]
pub enum WsStateInputConfig {
    Bool(WsInputBoolConfig),
    Float(WsInputFloatConfig),
    String(WsInputStringConfig),
}

#[derive(Debug)]
pub enum WsStateOutputConfig {
    Bool,
    Float,
    String,
}

#[derive(Debug)]
pub struct WsStateConfig {
    pub input_configs: HashMap<String, WsStateInputConfig>,
    pub output_configs: HashMap<String, WsStateOutputConfig>,
    pub channel_size: usize,
}

#[derive(Debug)]
pub struct WsServerConfig {
    pub state_config: WsStateConfig,
}

pub struct WsServer {
    pub handle: JoinHandle<()>,
    pub inputs: HashMap<String, WsInput>,
    pub outputs: HashMap<String, WsOutput>,
}

impl WsServer {
    pub async fn new(ws_server_config: WsServerConfig) -> WsServer {
        //create ws_state manager
        let state_config = ws_server_config.state_config;
        let ws_state = WsState::new(&state_config);
        let ws_state_cmd_tx = ws_state.cmd_tx.clone();

        //web socket mananger (subscribes to input and output updates, sends input updates)
        let ws_mgr = WsManager::new(ws_state.cmd_tx.clone());

        //get input subscription for ioc inputs
        let (subs_tx, subs_rx) = oneshot::channel();
        let subs_cmd = WsStateCmd::SubscribeInputs {
            subs_callback: subs_tx,
        };
        ws_state_cmd_tx.send(subs_cmd).await.unwrap();
        let subs = subs_rx.await.unwrap();

        //create individual WsInputs for each one
        let inputs = WsInput::from_subscription(subs);

        //tasks to write outputs to ws_state
        let outputs = WsOutput::from_config(ws_state_cmd_tx, state_config.output_configs);

        //service to server up static files
        let static_service = ServeDir::new("./assets")
            .append_index_html_on_directories(true)
            .not_found_service(ServeFile::new("./assets/404.html"));

        //router serves up static files or attemps to connect a websocket
        let router = Router::new()
            .fallback_service(static_service)
            .route("/ws", get(ws).with_state(ws_mgr.ws_sender.clone()))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(false)),
            );

        //start listening for http connections
        let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
        tracing::info!("listening on {}", addr);
        let handle = tokio::spawn(async move {
            //listen for sockets
            axum::Server::bind(&addr)
                .serve(router.into_make_service())
                .await
                .unwrap()
        });

        //return
        WsServer {
            handle,
            inputs,
            outputs,
        }
    }
}

//router function to handle websocket upgrade
async fn ws(
    ws: WebSocketUpgrade,
    State(ws_sender): State<mpsc::Sender<WebSocket>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move { ws_sender.send(socket).await.unwrap() })
}
