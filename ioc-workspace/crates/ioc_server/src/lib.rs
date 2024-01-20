pub mod error;
pub(crate) mod server;

use std::collections::HashMap;
use tokio::task::JoinHandle;
use tracing::{info,error};
use axum::response::IntoResponse;
use axum::routing::get;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tower_http::trace::DefaultMakeSpan;
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;

use crate::error::ServerBuildError;
use crate::server::{
    state::ServerState, 
    ServerInput, ServerInputBuilder, 
    ServerOutput, ServerOutputBuilder,
};

pub enum ServerInputConfig {
    Float {
        start: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Bool {
        start: bool,
    },
    String {
        start: String,
        max_length: u32,
    },
}

pub enum ServerOutputConfig {
    Float,
    Bool,
    String,
}

pub enum EndpointConfig<'a> {
    WebSocket {
        inputs: Vec<&'a str>,
        outputs: Vec<&'a str>,
    },
    Static {
        directory: String,
    },
}

pub struct ServerConfig<'a> {
    pub port: u16,
    pub root_context: &'a str,
    pub inputs: HashMap<&'a str, ServerInputConfig>,
    pub outputs: HashMap<&'a str, ServerOutputConfig>,
    pub endpoints: HashMap<&'a str, EndpointConfig<'a>>,
}

pub enum TypedInput {
    Float(ServerInput<f64>),
    Bool(ServerInput<bool>),
    String(ServerInput<String>),
}

pub enum TypedOutput {
    Float(ServerOutput<f64>),
    Bool(ServerOutput<bool>),
    String(ServerOutput<String>),
}

pub struct Server<'a> {
    pub handle: JoinHandle<()>,
    pub inputs: HashMap<&'a str, TypedInput>,
    pub outputs: HashMap<&'a str, TypedOutput>,
}

impl <'a> Server<'a> {
    pub fn try_build(cfg: ServerConfig<'a>) -> Result<Self, ServerBuildError> {
        let state = ServerState::try_build(&cfg.inputs, &cfg.outputs)?;

        let cmd_tx = state.cmd_tx;

        let mut inputs = HashMap::with_capacity(cfg.inputs.len());
        let mut outputs = HashMap::with_capacity(cfg.outputs.len());

        for (key, input_config) in cfg.inputs {
            let srv_input = ServerInputBuilder::try_build(&cmd_tx, input_config)?;
            inputs.insert(key, srv_input);
        }

        for (key, output_config) in cfg.outputs {
            let srv_output = ServerOutputBuilder::try_build(&cmd_tx, output_config)?;
            outputs.insert(key, srv_output);
        }

        //bind to 0.0.0.0 on the given port
        let socket_addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], cfg.port));

        let static_service = ServeDir::new("./assets")
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new("./assets/404.html"));


        let router_service = axum::routing::Router::new()
            .nest_service("/static", static_service)
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(false)),
            );

        let server_handle = tokio::spawn(async move {
            axum::Server::bind(&socket_addr) 
                .serve(router_service.into_make_service())
                .await
                .unwrap();
        });

        //let handle = futures::future::join_all(vec![server_handle, state.handle]).await.map(|_| {});

        Ok(Server {
            handle: server_handle,
            inputs,
            outputs,
        })
    }
}
