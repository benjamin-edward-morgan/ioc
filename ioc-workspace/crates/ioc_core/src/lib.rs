use tokio::sync::{broadcast, mpsc};

pub mod channel;
pub mod controller;
pub mod input;

pub struct InputSource<T> {
    pub start: T,
    pub rx: broadcast::Receiver<T>,
}

pub trait Input<T> {
    fn source(&self) -> InputSource<T>;
}

pub struct OutputSink<T> {
    pub tx: mpsc::Sender<T>,
}

pub trait Output<T> {
    fn sink(&self) -> OutputSink<T>;
}





