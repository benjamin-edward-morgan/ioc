use std::{collections::HashMap, sync::{Arc, Mutex}};

use crate::{error::IocBuildError, Input, InputKind, InputSource, Module, ModuleIO, Output, OutputKind, OutputSink};
use serde::Deserialize;
use tokio::{sync::{broadcast, mpsc}, task::JoinHandle};
use tracing::{info, warn};


#[derive(Debug, Deserialize)]
pub enum FeedbackItemConfig {
    Float{ start: f64} ,
}

///A FeedbackPipe implements Input and Output. Values sent to the Output are emitted by the input.
pub struct FeedbackPipe<T: Clone + Send + 'static> {
    last_value: Arc<Mutex<T>>,
    tx: mpsc::Sender<T>,
    rx: broadcast::Receiver<T>,
}

impl<T: Clone + Send + 'static> Clone for FeedbackPipe<T> {
    fn clone(&self) -> Self {
        Self {
            last_value: self.last_value.clone(),
            tx: self.tx.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}

/// Implementation of the `FeedbackPipe` struct.
    /// Spawns a new `FeedbackPipe` with a starting value.
    ///
    /// # Arguments
    ///
    /// * `start` - The initial value for the `FeedbackPipe`.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the spawned `FeedbackPipe` and a `JoinHandle` to the spawned task.
    ///
    /// # Example
    ///
    /// ```
    /// use tokio::task::JoinHandle;
    /// use tokio::sync::{broadcast, mpsc};
    /// use std::sync::{Arc, Mutex};
    ///
    /// let start_value = 0;
    /// let (pipe, join_handle) = FeedbackPipe::spawn(start_value);
    /// ```
impl <T: Clone + Send + 'static> FeedbackPipe<T> {
    // Create a new FeedbackPipe with a starting value. Returns the FeedbackPipe and a JoinHandle to the spawned task.
    pub fn spawn(start: T) -> (Self, JoinHandle<()>) {
        let (tx, mut out_rx) = mpsc::channel::<T>(10);
        let (in_tx, rx) = broadcast::channel(10);
        let last_value = Arc::new(Mutex::new(start));
        let last_value_clone = last_value.clone();
        let join_handle = tokio::spawn(async move {
            while let Some(new_value) = out_rx.recv().await {
                let mut current_value = match last_value_clone.lock() {
                    Ok(v) => v,
                    Err(poisoned) => poisoned.into_inner(),
                };
                *current_value = new_value.clone();
                if let Err(err) = in_tx.send(new_value) {
                    warn!("send error in feedback pipe: {}", err);
                    break;
                }
            }
            info!("shutting down feedback pipe!");
        });
        let pipe = Self {
            last_value,
            tx,
            rx,
        };
        (pipe, join_handle)
    }

}


impl <T: Clone + Send> Input<T> for FeedbackPipe<T> {
    fn source(&self) -> InputSource<T> {
        let value = match self.last_value.lock() {
            Ok(v) => v,
            Err(poisoned) => poisoned.into_inner(),
        };
        InputSource{
            start: value.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}

impl <T: Clone + Send> Output<T> for FeedbackPipe<T> {
    fn sink(&self) -> OutputSink<T> {
        OutputSink{
            tx: self.tx.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct FeedbackConfig {
    items: HashMap<String, FeedbackItemConfig>,
}

/// A Feedback module is a collection of FeedbackPipes. Each FeedbackPipe is an Input and an Output.
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

impl Module for Feedback {
    type Config = FeedbackConfig;
    
    async fn try_build(cfg: &Self::Config) -> Result<Self, IocBuildError> {
        let mut inputs = HashMap::with_capacity(cfg.items.len());
        let mut outputs = HashMap::with_capacity(cfg.items.len());
        let mut join_handles = Vec::with_capacity(cfg.items.len());

        for (name, item_cfg) in &cfg.items {
            match item_cfg {
                FeedbackItemConfig::Float{ start } => {
                    let (feedback, join_handle) = FeedbackPipe::spawn(*start);
                    inputs.insert(name.clone(), InputKind::float(feedback.clone()));
                    outputs.insert(name.clone(), OutputKind::float(feedback));
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

