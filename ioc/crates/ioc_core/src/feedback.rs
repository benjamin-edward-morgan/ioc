use std::collections::HashMap;

use crate::{error::IocBuildError, Input, InputKind, Module, ModuleIO, Output, OutputKind, Value};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

/// Configuration for a FeedbackItem. Contains a start value.
#[derive(Debug, Deserialize)]
pub enum FeedbackItemConfig {
    String{ start: String },
    Binary{ start: Vec<u8> },
    Bool{ start: bool },
    Float{ start: f64} ,
    Array{ start: Vec<Value> },
    Object{ start: HashMap<String, Value> },
}

/// Configuration for a Feedback module. Contains a map of FeedbackItemConfigs.
/// 
/// Each FeedbackItemConfig contains a start value and will create one Input and one corresponding Output.
#[derive(Deserialize, Debug)]
pub struct FeedbackConfig {
    items: HashMap<String, FeedbackItemConfig>,
}

/// A Feedback module is a collection of FeedbackPipes. Each FeedbackPipe has an Input and an Output.
/// 
/// Each output sends any values received to the corresponding input.
/// 
/// The join_handles of each feedback_pipe are joined together and awaited in the Feedback module's join_handle.
pub struct Feedback {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
    pub outputs: HashMap<String, OutputKind>,
}

impl From<Feedback> for ModuleIO {
    fn from(feedback: Feedback) -> Self {
        ModuleIO { 
            join_handle: feedback.join_handle, 
            inputs: feedback.inputs, 
            outputs: feedback.outputs,
        }
    }
}

///Spawns a feedback pipe with the given start value. Returns the Input, Output, and JoinHandle.
fn spawn_feedback_pipe<T: Send + Sync + 'static>(start: T, cancel_token: CancellationToken) -> (Input<T>, Output<T>, JoinHandle<()>) {
    let (input, tx) = Input::new(start);
    let (output, mut rx) = Output::new();
    let handle = tokio::spawn(async move {
        tokio::select!{
            _ = cancel_token.cancelled() => {}
            _ = tokio::spawn(async move {
                while let Some(new_value) = rx.recv().await {
                    if let Err(err) = tx.send(new_value) {
                        warn!("Error sending to Input from feedback pipe: {}", err);
                        break;
                    }
                }
            }) => {}
        }
        debug!("feedback pipe shut down!")
    });
    (input, output, handle)
}

/// Implementation of the `Module` trait for the `Feedback` struct.
impl Module for Feedback {
    type Config = FeedbackConfig;
    
    /// Asynchronously tries to build a `Feedback` instance based on the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `cfg` - The configuration for building the `Feedback` instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the built `Feedback` instance if successful, or an `IocBuildError` if an error occurs.
    async fn try_build(cfg: &Self::Config, cancel_token: CancellationToken) -> Result<Self, IocBuildError> {
        let mut inputs = HashMap::with_capacity(cfg.items.len());
        let mut outputs = HashMap::with_capacity(cfg.items.len());
        let mut join_handles = Vec::with_capacity(cfg.items.len());

        for (name, item_cfg) in &cfg.items {
            match item_cfg {
                FeedbackItemConfig::String{ start} => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::String(input));
                    outputs.insert(name.clone(), OutputKind::String(output));
                    join_handles.push(join_handle);
                },
                FeedbackItemConfig::Binary { start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::Binary(input));
                    outputs.insert(name.clone(), OutputKind::Binary(output));
                    join_handles.push(join_handle);
                },
                FeedbackItemConfig::Float{ start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::Float(input));
                    outputs.insert(name.clone(), OutputKind::Float(output));
                    join_handles.push(join_handle);
                },
                FeedbackItemConfig::Bool{ start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::Bool(input));
                    outputs.insert(name.clone(), OutputKind::Bool(output));
                    join_handles.push(join_handle);
                },
                FeedbackItemConfig::Array { start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::Array(input));
                    outputs.insert(name.clone(), OutputKind::Array(output));
                    join_handles.push(join_handle);
                },
                FeedbackItemConfig::Object { start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(start.clone(), cancel_token.clone());
                    inputs.insert(name.clone(), InputKind::Object(input));
                    outputs.insert(name.clone(), OutputKind::Object(output));
                    join_handles.push(join_handle);
                },       
            }
        }

        let join_handle = tokio::task::spawn(async move {
            for join_handle in join_handles {
                join_handle.await.unwrap();
            }
        });

        Ok(Feedback {
            join_handle,
            inputs,
            outputs,
        })
    }
}

