use crate::{Output, OutputSink};
use std::{collections::HashMap, fmt::Debug};
use tokio::sync::mpsc;
use tracing::info;

use super::state::{WsStateCmd, WsStateValue};
use super::WsStateOutputConfig;

#[derive(Debug)]
pub enum WsOutput {
    Bool { output: WsGenOutput<bool> },
    Float { output: WsGenOutput<f64> },
    String { output: WsGenOutput<String> },
}

impl WsOutput {
    pub fn from_config(
        state_cmd_tx: mpsc::Sender<WsStateCmd>,
        output_configs: HashMap<String, WsStateOutputConfig>,
    ) -> HashMap<String, WsOutput> {
        let mut outputs = HashMap::with_capacity(10);

        output_configs.iter().for_each(|(k, v)| {
            match *v {
                WsStateOutputConfig::Bool => {
                    let (tx, mut rx) = mpsc::channel(10);
                    let output = WsGenOutput::new(tx);
                    outputs.insert(k.to_string(), WsOutput::Bool { output });

                    let state_cmd_tx = state_cmd_tx.clone();
                    let k = k.to_string();
                    tokio::spawn(async move {
                        while let Some(b) = rx.recv().await {
                            let mut state = HashMap::with_capacity(1);
                            state.insert(k.clone(), WsStateValue::Bool { b });
                            let state_cmd = WsStateCmd::SetOutputs { state };
                            state_cmd_tx.send(state_cmd).await.unwrap();
                        }
                    });
                }
                WsStateOutputConfig::Float => {
                    let (tx, mut rx) = mpsc::channel(10);
                    let output = WsGenOutput::new(tx);
                    outputs.insert(k.to_string(), WsOutput::Float { output });

                    let state_cmd_tx = state_cmd_tx.clone();
                    let k = k.to_string();
                    tokio::spawn(async move {
                        while let Some(f) = rx.recv().await {
                            let mut state = HashMap::with_capacity(1);
                            state.insert(k.clone(), WsStateValue::Float { f });
                            let state_cmd = WsStateCmd::SetOutputs { state };
                            state_cmd_tx.send(state_cmd).await.unwrap();
                        }
                        info!("ws output done!");
                    });
                }
                WsStateOutputConfig::String => {
                    let (tx, mut rx) = mpsc::channel(10);
                    let output = WsGenOutput::new(tx);
                    outputs.insert(k.to_string(), WsOutput::String { output });

                    let state_cmd_tx = state_cmd_tx.clone();
                    let k = k.to_string();
                    tokio::spawn(async move {
                        while let Some(s) = rx.recv().await {
                            let mut state = HashMap::with_capacity(1);
                            state.insert(k.clone(), WsStateValue::String { s });
                            let state_cmd = WsStateCmd::SetOutputs { state };
                            state_cmd_tx.send(state_cmd).await.unwrap();
                        }
                    });
                }
            };
        });

        outputs
    }
}

#[derive(Debug)]
pub struct WsGenOutput<T> {
    tx: mpsc::Sender<T>,
}

impl<T> WsGenOutput<T> {
    pub fn new(tx: mpsc::Sender<T>) -> WsGenOutput<T> {
        WsGenOutput { tx }
    }
}

impl Output<bool> for WsGenOutput<bool> {
    fn sink(&self) -> OutputSink<bool> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

impl Output<f64> for WsGenOutput<f64> {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

impl Output<String> for WsGenOutput<String> {
    fn sink(&self) -> OutputSink<String> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}
