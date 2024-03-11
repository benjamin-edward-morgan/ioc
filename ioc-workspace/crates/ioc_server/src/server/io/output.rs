use crate::server::state::{StateCmd,StateUpdate,ServerOutputState};

use tokio::task::JoinHandle;
use tracing::info;
use ioc_core::{Output, OutputSink};
use std::collections::HashMap;

use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ServerOutput<T: Send + 'static> {
    pub handle: JoinHandle<()>,
    sink_tx: mpsc::Sender<T>,
}

impl <T: Send + 'static> ServerOutput<T> {
    pub(crate) fn new(
        key: &str,
        cmd_tx: mpsc::Sender<StateCmd>,
        channel_size: usize,
        transform: fn(T) -> ServerOutputState,
    ) -> Self {
        
        let (sink_tx, mut sink_rx) = mpsc::channel(channel_size);

        let cmd_tx = cmd_tx.clone();
        let handle_key = key.to_string();
        let handle = tokio::spawn(async move {
            while let Some(t) = sink_rx.recv().await {
                let update = StateUpdate {
                    inputs: HashMap::new(),
                    outputs: HashMap::from([
                        (handle_key.to_string(), transform(t))
                    ])
                };

                let cmd = StateCmd::Update(update);

                if let Err(err) = cmd_tx.send(cmd).await {
                    panic!("error sending state update cmd in server output {:?}", err);
                }
            }
            info!("server output shutting down!");
        });

        Self{
            handle,
            sink_tx,
        }
    }
}

impl<T: Send + 'static> Output<T> for ServerOutput<T> {
    fn sink(&self) -> OutputSink<T> {
        OutputSink{
            tx: self.sink_tx.clone()
        }
    }
}
