use crate::{Input, InputSource, Output, OutputSink};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tracing::warn;

pub struct Channel<T> {
    start: T,
    tx: mpsc::Sender<T>,
    rx: broadcast::Receiver<T>,
    pub handle: JoinHandle<()>,
}

impl<T> Channel<T>
where
    T: Send + Clone + 'static,
{
    pub fn new(start: T) -> Channel<T> {
        let (i_tx, i_rx) = broadcast::channel(16);
        let (o_tx, mut o_rx) = mpsc::channel(16);

        let handle = tokio::spawn(async move {
            while let Some(x) = o_rx.recv().await {
                if i_tx.send(x).is_err() {
                    warn!("outin can't send!");
                    break;
                }
            }
            warn!("OutputInput is done!");
        });

        Channel {
            start,
            tx: o_tx,
            rx: i_rx,
            handle,
        }
    }
}

impl<T> Input<T> for Channel<T>
where
    T: Send + Clone + 'static,
{
    fn source(&self) -> InputSource<T> {
        InputSource {
            start: self.start.clone(),
            rx: self.rx.resubscribe(),
        }
    }
}

impl<T> Output<T> for Channel<T>
where
    T: Send + Clone + 'static,
{
    fn sink(&self) -> OutputSink<T> {
        OutputSink {
            tx: self.tx.clone(),
        }
    }
}