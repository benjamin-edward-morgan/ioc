use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::{Output, OutputSink};
use std::fmt::Debug;

pub struct ConsoleOutput<T: Debug + Send + 'static> {
    pub handle: JoinHandle<()>,
    tx: mpsc::Sender<T>,
}

impl<T: Debug + Send> ConsoleOutput<T> {
    pub fn new(name: &str) -> ConsoleOutput<T> {
        let (tx, mut rx) = mpsc::channel(128);

        let name = name.to_string();

        let handle = tokio::spawn(async move {
            while let Some(f) = rx.recv().await {
                info!("{}: {:?}", name, f);
            }
            warn!("console output shutting down");
        });

        ConsoleOutput { handle, tx }
    }
}

impl Output<f64> for ConsoleOutput<f64> {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

impl Output<String> for ConsoleOutput<String> {
    fn sink(&self) -> OutputSink<String> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

impl Output<bool> for ConsoleOutput<bool> {
    fn sink(&self) -> OutputSink<bool> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ConsoleOutputConfig {
    name: String,
}

impl From<&ConsoleOutputConfig> for Box<dyn Output<f64>> {
    fn from(val: &ConsoleOutputConfig) -> Self {
        let output = ConsoleOutput::new(val.name.as_str());
        Box::new(output)
    }
}

impl From<&ConsoleOutputConfig> for Box<dyn Output<bool>> {
    fn from(val: &ConsoleOutputConfig) -> Self {
        let output = ConsoleOutput::new(val.name.as_str());
        Box::new(output)
    }
}

impl From<&ConsoleOutputConfig> for Box<dyn Output<String>> {
    fn from(val: &ConsoleOutputConfig) -> Self {
        let output = ConsoleOutput::new(val.name.as_str());
        Box::new(output)
    }
}
