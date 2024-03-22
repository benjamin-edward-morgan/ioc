use std::collections::HashMap;

use crate::{error::IocBuildError, Input, InputKind, Module, ModuleIO, Output, OutputKind};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{debug, warn};


#[derive(Debug, Deserialize)]
pub enum FeedbackItemConfig {
    Float{ start: f64} ,
}

#[derive(Deserialize, Debug)]
pub struct FeedbackConfig {
    items: HashMap<String, FeedbackItemConfig>,
}

/// A Feedback module is a collection of FeedbackPipes. Each FeedbackPipe is an Input and an Output, such that 
/// the Output sends its value to the Input.
/// the join_handles of each feedback_pipe are joined together and awaited in the Feedback module's join_handle.
pub struct Feedback {
    join_handle: JoinHandle<()>,
    inputs: HashMap<String, InputKind>,
    outputs: HashMap<String, OutputKind>,
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

fn spawn_feedback_pipe<T: Send + Sync + 'static>(start: T) -> (Input<T>, Output<T>, JoinHandle<()>) {
    let (input, tx) = Input::new(start);
    let (output, mut rx) = Output::new();
    let handle = tokio::spawn(async move {
        while let Some(new_value) = rx.recv().await {
            if let Err(err) = tx.send(new_value) {
                warn!("Error sending to Input from feedback pipe: {}", err);
                break;
            }
        }
        debug!("feedback pipe shut down!")
    });
    (input, output, handle)
}

impl Module for Feedback {
    type Config = FeedbackConfig;
    
    async fn try_build(cfg: &Self::Config) -> Result<Self, IocBuildError> {
        let mut inputs = HashMap::with_capacity(cfg.items.len());
        let mut outputs = HashMap::with_capacity(cfg.items.len());
        let mut join_handles = Vec::with_capacity(cfg.items.len());

        for (name, item_cfg) in &cfg.items {
            match item_cfg {
                FeedbackItemConfig::Float{ start } => {
                    let (input, output, join_handle) = spawn_feedback_pipe(*start);
                    inputs.insert(name.clone(), InputKind::Float(input));
                    outputs.insert(name.clone(), OutputKind::Float(output));
                    join_handles.push(join_handle);
                }
            }
        }


        Ok(Feedback {
            join_handle: tokio::task::spawn(async move {
                for join_handle in join_handles {
                    join_handle.await.unwrap();
                }
            }),
            inputs,
            outputs,
        })
    }
}

