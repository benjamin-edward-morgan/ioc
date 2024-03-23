use crate::server::state::{StateCmd, StateUpdate};
use crate::server::state::{ServerInputState, ServerOutputState};
use crate::{ServerInputConfig, ServerOutputConfig};

use ioc_core::error::IocBuildError;
use ioc_core::{Input, InputKind, Output, OutputKind};
use tracing::{debug, warn};
use std::collections::{HashMap, HashSet};
use tokio::sync::{mpsc, oneshot};

pub(crate) struct ServerIoBuilder {
    pub cmd_tx: mpsc::Sender<StateCmd>,
}

impl ServerIoBuilder {
    pub(crate) async fn try_build_input(
        &self,
        key: &str,
        config: &ServerInputConfig,
    ) -> Result<InputKind, IocBuildError> {
        let (subs_tx, subs_rx) = oneshot::channel();
        let subs_cmd = StateCmd::Subscribe {
            callback: subs_tx, 
            inputs: HashSet::from([key.to_string()]), 
            outputs: HashSet::new() 
        };
        self.cmd_tx.send(subs_cmd ).await.unwrap();
        let subs = subs_rx.await.unwrap();
        match config {
                ServerInputConfig::Bool { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::Bool { value }) => Ok(*value),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected Bool input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::Bool { value } = value {
                                    tx.send(*value).unwrap();
                                } else {
                                    warn!("Expected Bool input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::Bool(input))
                },
                ServerInputConfig::Float { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::Float { value, .. }) => Ok(*value),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected Float input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::Float { value, .. } = value {
                                    tx.send(*value).unwrap();
                                } else {
                                    warn!("Expected Float input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::Float(input))
                },
                ServerInputConfig::String { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::String { value, .. }) => Ok(value.clone()),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected String input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::String { value, .. } = value {
                                    tx.send(value.clone()).unwrap();
                                } else {
                                    warn!("Expected String input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::String(input))
                },
                ServerInputConfig::Binary { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::Binary { value, .. }) => Ok(value.clone()),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected Binary input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::Binary { value, .. } = value {
                                    tx.send(value.clone()).unwrap();
                                } else {
                                    warn!("Expected Binary input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::Binary(input))
                },
                ServerInputConfig::Array { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::Array { value }) => Ok(value.clone()),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected Array input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::Array { value } = value {
                                    tx.send(value.clone()).unwrap();
                                } else {
                                    warn!("Expected Array input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::Array(input))
                },
                ServerInputConfig::Object { .. } => {
                    let start = match subs.start.inputs.get(key) {
                        Some(ServerInputState::Object { value }) => Ok(value.clone()),
                        Some(_) => Err(IocBuildError::from_string(format!("Expected Object input for key: {}", key))),
                        None => Err(IocBuildError::from_string(format!("No input found for key: {}", key))),
                    }?;
                    let (input, tx) = Input::new(start);
                    let mut subs_rx = subs.update_rx;
                    let key = key.to_owned();
                    tokio::spawn(async move {
                        while let Ok(update) = subs_rx.recv().await {
                            if let Some(value) = update.inputs.get(&key) {
                                if let ServerInputState::Object { value } = value {
                                    tx.send(value.clone()).unwrap();
                                } else {
                                    warn!("Expected Object input for key: {}", key);
                                }
                            }
                        }
                        debug!("Server input shutting down!");
                    });
                    Ok(InputKind::Object(input))
                },
        }
    }

    pub(crate) async fn try_build_output(
        &self,
        key: &str,
        config: &ServerOutputConfig,
    ) -> Result<OutputKind, IocBuildError> {
        
        match config {
            ServerOutputConfig::Bool => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::Bool{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::Bool(output))
            },
            ServerOutputConfig::Float => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::Float{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::Float(output))
            },
            ServerOutputConfig::String => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::String{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::String(output))
            },
            ServerOutputConfig::Binary => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::Binary{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::Binary(output))
            },
            ServerOutputConfig::Array => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::Array{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::Array(output))
            },
            ServerOutputConfig::Object => {
                let (output, mut rx) = Output::new();
                let cmd_tx = self.cmd_tx.clone();
                let key = key.to_owned();
                tokio::spawn(async move {
                    while let Some(value) = rx.recv().await {
                        cmd_tx.send(StateCmd::Update(StateUpdate{
                            inputs: HashMap::new(),
                            outputs: HashMap::from([(key.to_string(), ServerOutputState::Object{ value: Some(value) })]),
                        })).await.unwrap();
                    }
                    debug!("Server output shutting down!");
                });
                Ok(OutputKind::Object(output))
            },
        }
    }
}
