pub mod error;
pub(crate) mod server;

use std::collections::HashMap;
use std::rc::Rc;
use ioc_core::InputKind;
use ioc_core::ModuleIO;
use ioc_core::ModuleBuilder;
use ioc_core::OutputKind;
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
        max_length: u32,
    },
}

#[derive(Deserialize, Debug)]
pub enum ServerOutputConfig {
    Float,
    Bool,
    String,
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
        frames_output: String,
    }
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

#[derive(Debug)]
pub enum TypedInput {
    Float(ServerInput<f64>),
    Bool(ServerInput<bool>),
    String(ServerInput<String>),
}

#[derive(Debug)]
pub enum TypedOutput {
    Float(ServerOutput<f64>),
    Bool(ServerOutput<bool>),
    String(ServerOutput<String>),
}

pub struct Server {
    pub handle: JoinHandle<()>,
    pub inputs: HashMap<String, TypedInput>,
    pub outputs: HashMap<String, TypedOutput>,
}

impl From<Server> for ModuleIO {
    fn from(server: Server) -> Self {
        let mut inputs = HashMap::with_capacity(server.inputs.len());
        for (k, input) in server.inputs {
            let ik = match input {
                TypedInput::String(str) => InputKind::String(Box::new(str)),
                TypedInput::Float(float) => InputKind::Float(Box::new(float)),
                TypedInput::Bool(bool) => InputKind::Bool(Box::new(bool)),
            };
            inputs.insert(k.to_string(), ik);
        }

        let mut outputs = HashMap::with_capacity(server.outputs.len());
        for (k, output) in server.outputs {
            let ok = match output {
                TypedOutput::String(str) => OutputKind::String(Box::new(str)),
                TypedOutput::Float(float) => OutputKind::Float(Box::new(float)),
                TypedOutput::Bool(bool) => OutputKind::Bool(Box::new(bool)),
                
            };
            outputs.insert(k.to_string(), ok);
        }


        ModuleIO { 
            join_handle: server.handle,
            inputs, 
            outputs,
        }
    }
}

impl ModuleBuilder for Server {
    type Config = ServerConfig;
    type Error = ServerBuildError;

    async fn try_build(cfg: &ServerConfig) -> Result<Self, ServerBuildError> {
        info!("building server state ...");

        //global state
        let state = ServerState::try_build(cfg.state_channel_size.unwrap_or(16), &cfg.inputs, &cfg.outputs)?;
        let cmd_tx = state.cmd_tx;

        info!("building server inputs, outputs ...");
        //create the inputs and outputs
        let mut inputs = HashMap::with_capacity(cfg.inputs.len());
        let mut outputs = HashMap::with_capacity(cfg.outputs.len());
        info!("building server io builder ...");

        let io_builder = ServerIoBuilder {
            cmd_tx: cmd_tx.clone(),
            channel_size: cfg.io_channel_size.unwrap_or(16),
        };
        info!("building server inputs ...");
        for (key, input_config) in cfg.inputs.iter() {
            let srv_input = io_builder.try_build_input(&key, input_config).await?;
            inputs.insert(key.to_string(), srv_input);
        }
        info!("building server outputs ...");
        for (key, output_config) in cfg.outputs.iter() {
            let srv_output = io_builder.try_build_output(&key, output_config).await?;
            outputs.insert(key.to_string(), srv_output);
        }

        //build router service from endpoint configs
        info!("building routers ...");
        let mut router_service = axum::routing::Router::new();
        for (key, ep_config) in cfg.endpoints.iter() {
            info!("building router {} ...", key);
            let endpoint: Endpoint = Endpoint::try_build(&cmd_tx, ep_config);
            router_service = endpoint.apply(&key, router_service);
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


// impl ModuleBuilder for Server {


//     fn join_handle(self) -> JoinHandle<()> {
//         self.handle
//     }

//     fn inputs<'a>(&'a self) -> HashMap<String, InputKind<'a>> {
        
//     }

//     fn input(&self, name: &str) -> Option<InputKind> {
//         self.inputs.get(name).map(|input| {
//             match input {
//                 TypedInput::String(str_input) => InputKind::String(str_input),
//                 TypedInput::Float(float_input) => InputKind::Float(float_input),
//                 TypedInput::Bool(bool_input) => InputKind::Bool(bool_input),    
//             }
//         })
//     }
//     fn output(&self, name: &str) -> Option<OutputKind> {
//         self.outputs.get(name).map(|output| {
//             match output {
//                 TypedOutput::String(str_out) => OutputKind::String(str_out),
//                 TypedOutput::Float(float_out) => OutputKind::Float(float_out),
//                 TypedOutput::Bool(bool_out) => OutputKind::Bool(bool_out),
//             }
//         })
//     }
// }
