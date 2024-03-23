pub(crate) mod server;

use std::collections::HashMap;

use ioc_core::error::IocBuildError;
use ioc_core::InputKind;
use ioc_core::Module;
use ioc_core::ModuleIO;
use ioc_core::OutputKind;
use ioc_core::Value;
use tokio::join;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::info;

use std::net::SocketAddr;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;

use serde::Deserialize;

use crate::server::{
    endpoint::Endpoint, io::ServerIoBuilder,
    state::ServerState,
};

#[derive(Deserialize, Debug)]
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
        max_length: usize,
    },
    Binary {
        start: Vec<u8>,
    },
    Array {
        start: Vec<Value>,
    },
    Object {
        start: HashMap<String, Value>,
    },
}

#[derive(Deserialize, Debug)]
pub enum ServerOutputConfig {
    Float,
    Bool,
    String,
    Binary,
    Array,
    Object
}

#[derive(Deserialize, Debug)]
pub enum EndpointConfig {
    WebSocket {
        inputs: Vec<String>,
        outputs: Vec<String>,
    },
    Static {
        directory: String,
    },
    Mjpeg {
        output: String,
    },
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub root_context: String,
    pub inputs: HashMap<String, ServerInputConfig>,
    pub outputs: HashMap<String, ServerOutputConfig>,
    pub endpoints: HashMap<String, EndpointConfig>,
    pub state_channel_size: Option<usize>,
    pub io_channel_size: Option<usize>,
}



pub struct Server {
    pub handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
    pub outputs: HashMap<String, OutputKind>,
}

impl From<Server> for ModuleIO {
    fn from(server: Server) -> Self {
        ModuleIO {
            join_handle: server.handle,
            inputs: server.inputs,
            outputs: server.outputs,
        }
    }
}

impl Module for Server {
    type Config = ServerConfig;

    async fn try_build(cfg: &ServerConfig) -> Result<Self, IocBuildError> {
        debug!("building server state ...");

        //global state
        let state = ServerState::try_build(
            cfg.state_channel_size.unwrap_or(16),
            &cfg.inputs,
            &cfg.outputs,
        )?;
        let cmd_tx = state.cmd_tx;

        debug!("building server inputs, outputs ...");
        //create the inputs and outputs
        let mut inputs = HashMap::with_capacity(cfg.inputs.len());
        let mut outputs = HashMap::with_capacity(cfg.outputs.len());
        info!("building server io builder ...");

        let io_builder = ServerIoBuilder {
            cmd_tx: cmd_tx.clone(),
        };
        debug!("building server inputs ...");
        for (key, input_config) in cfg.inputs.iter() {
            let srv_input = io_builder.try_build_input(key, input_config).await?;
            inputs.insert(key.to_string(), srv_input);
        }
        debug!("building server outputs ...");
        for (key, output_config) in cfg.outputs.iter() {
            let srv_output = io_builder.try_build_output(key, output_config).await?;
            outputs.insert(key.to_string(), srv_output);
        }

        //build router service from endpoint configs
        debug!("building routers ...");
        let mut router_service = axum::routing::Router::new();
        for (key, ep_config) in cfg.endpoints.iter() {
            debug!("building router {} ...", key);
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

        let join_handle = tokio::spawn(async move {
            let _ = join!(server_handle, state.handle);
        });

        Ok(Server {
            handle: join_handle,
            inputs,
            outputs,
        })
    }
}
