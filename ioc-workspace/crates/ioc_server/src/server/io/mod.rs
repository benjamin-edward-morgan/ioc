pub mod input;
pub mod output;

use crate::error::ServerBuildError;
use crate::server::state::{ServerInputState, ServerOutputState};
use crate::server::state::StateCmd;
use crate::{ServerInputConfig, ServerOutputConfig, TypedInput, TypedOutput};

use input::ServerInput;
use output::ServerOutput;
use tokio::sync::{mpsc, oneshot};
use std::collections::HashSet;

pub(crate) struct ServerIoBuilder {
    pub cmd_tx: mpsc::Sender<StateCmd>,
    pub channel_size: usize,
}

impl ServerIoBuilder {
    pub(crate) async fn try_build_input(
        &self,
        key: &str,
        config: &ServerInputConfig,
    ) -> Result<TypedInput, ServerBuildError> {
        let (subs_tx, subs_rx) = oneshot::channel();

        let cmd = StateCmd::Subscribe{
            callback: subs_tx, 
            inputs: HashSet::from([key.to_string()]), 
            outputs: HashSet::new()
        };

        if let Err(err) = self.cmd_tx.send(cmd).await {
            return Err(ServerBuildError::new(format!(
                "error sending subscription command {}",
                err
            )));
        }

        let subs = match subs_rx.await {
            Ok(subs) => subs,
            Err(err) => {
                return Err(ServerBuildError::new(format!(
                    "Error getting subscription for ServerInput {:?}",
                    err
                )));
            }
        };

        let start = subs
            .start
            .inputs
            .get(key)
            .ok_or(ServerBuildError::new(format!(
                "subscription start did not contain key {:?}",
                key
            )))?;

        let typed_input = match (config, start) {
            (ServerInputConfig::Float { .. }, ServerInputState::Float { value, .. }) => {
                let input = ServerInput::new(
                    key.to_string(), 
                    self.channel_size, 
                    *value, 
                    subs.update_rx,
                    | state: &ServerInputState | {
                        match state {
                            ServerInputState::Float{ value, .. } => Some(*value),
                            _ => None
                        }
                    },
                );
                Ok(TypedInput::Float(input))
            }
            (ServerInputConfig::Bool { .. }, ServerInputState::Bool { value }) => {
                let input = ServerInput::new(
                    key.to_string(), 
                    self.channel_size, 
                    *value, 
                    subs.update_rx,
                    | state: &ServerInputState | {
                        match state {
                            ServerInputState::Bool{ value } => Some(*value),
                            _ => None
                        }
                    },
                );
                Ok(TypedInput::Bool(input))
            }
            (ServerInputConfig::String { .. }, ServerInputState::String { value, .. }) => {
                let input = ServerInput::new(
                    key.to_string(),
                    self.channel_size,
                    value.clone(),
                    subs.update_rx,
                    | state: &ServerInputState | {
                        match state {
                            ServerInputState::String{ value, .. } => Some(value.to_string()),
                            _ => None
                        }
                    },
                );
                Ok(TypedInput::String(input))
            }
            (_, _) => Err(ServerBuildError::new(
                "Got mismatched typed attempting to construct ServerInput.".to_string(),
            )),
        }?;

        Ok(typed_input)
    }

    pub(crate) async fn try_build_output(
        &self,
        key: &str,
        config: &ServerOutputConfig,
    ) -> Result<TypedOutput, ServerBuildError> {
        let typed_output = match config {
            ServerOutputConfig::Float => {
                let output = ServerOutput::new(
                    key,
                    self.cmd_tx.clone(),
                    self.channel_size,
                    |f: f64| ServerOutputState::Float{value: Some(f)},
                );
                TypedOutput::Float(output)
            },
            ServerOutputConfig::Bool => {
                let output = ServerOutput::new(
                    key,
                    self.cmd_tx.clone(),
                    self.channel_size,
                    |b: bool| ServerOutputState::Bool{value: Some(b)},
                );
                TypedOutput::Bool(output)
            },
            ServerOutputConfig::String => {
                let output = ServerOutput::new(
                    key,
                    self.cmd_tx.clone(),
                    self.channel_size,
                    |s: String| ServerOutputState::String{value: Some(s)},
                );
                TypedOutput::String(output)
            },
        };

        Ok(typed_output)
    }
}
