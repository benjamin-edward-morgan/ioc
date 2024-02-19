pub mod error;
pub(crate) mod server;

use std::collections::HashMap;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::info;

use std::net::SocketAddr;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;

use serde::Deserialize;

use crate::error::ServerBuildError;
use crate::server::{
    endpoint::Endpoint, io::input::ServerInput, io::output::ServerOutput, io::ServerIoBuilder,
    state::ServerState,
};

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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
        directory: &'a str,
    },
    Mjpeg {
        frames: watch::Receiver<Vec<u8>>,
    }
}

pub struct ServerConfig<'a> {
    pub port: u16,
    pub root_context: &'a str,
    pub inputs: HashMap<&'a str, ServerInputConfig>,
    pub outputs: HashMap<&'a str, ServerOutputConfig>,
    pub endpoints: HashMap<&'a str, EndpointConfig<'a>>,
    pub state_channel_size: usize,
    pub io_channel_size: usize,
}

#[derive(Debug)]
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

impl<'a> Server<'a> {
    pub async fn try_build(cfg: ServerConfig<'a>) -> Result<Self, ServerBuildError> {
        info!("building server state!");

        //global state
        let state = ServerState::try_build(cfg.state_channel_size, &cfg.inputs, &cfg.outputs)?;
        let cmd_tx = state.cmd_tx;

        println!("server inputs, outputs ...");
        //create the inputs and outputs
        let mut inputs = HashMap::with_capacity(cfg.inputs.len());
        let mut outputs = HashMap::with_capacity(cfg.outputs.len());
        println!("server io builder ...");

        let io_builder = ServerIoBuilder {
            cmd_tx: cmd_tx.clone(),
            channel_size: cfg.io_channel_size,
        };
        println!("server inputs ...");
        for (key, input_config) in cfg.inputs {
            let srv_input = io_builder.try_build_input(key, input_config).await?;
            inputs.insert(key, srv_input);
        }
        println!("server outputs ...");
        for (key, output_config) in cfg.outputs {
            let srv_output = io_builder.try_build_output(key, output_config).await?;
            outputs.insert(key, srv_output);
        }

        //build router service from endpoint configs
        println!("building routers!");
        let mut router_service = axum::routing::Router::new();
        for (key, ep_config) in cfg.endpoints {
            println!("building router {} ...", key);
            let endpoint: Endpoint = Endpoint::try_build(&cmd_tx, ep_config);
            router_service = endpoint.apply(key, router_service);
        }
        router_service = router_service.layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(false)),
        );

        //bind to 0.0.0.0 on the given port
        let socket_addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], cfg.port));

        //start handling requests
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
