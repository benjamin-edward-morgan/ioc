use tokio::sync::{broadcast, mpsc};

mod channel;
pub mod config;
mod controller;
mod input;
mod output;
mod sim;

#[cfg(feature = "ws-server")]
mod ws;

#[cfg(feature = "rpi")]
mod rpi;

#[cfg(feature = "devices")]
mod devices;

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
