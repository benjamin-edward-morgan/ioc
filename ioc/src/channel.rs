use crate::{InputSource,OutputSink, Input, Output, config::ChannelBox};
use serde::Deserialize;
use tokio::{task::JoinHandle, sync::{broadcast, mpsc}};
use tracing::warn;



pub struct Channel<T> {
    start: T,
    tx: mpsc::Sender<T>,
    rx: broadcast::Receiver<T>,
    pub handle: JoinHandle<()>
}

impl <T> Channel<T> 
where
    T: Send + Clone + 'static
{
    pub fn new(start: T) -> Channel<T> {

        let (i_tx, i_rx) = broadcast::channel(16);
        let (o_tx, mut o_rx) = mpsc::channel(16);

        let handle = tokio::spawn(async move {
            while let Some(x) = o_rx.recv().await {
                if i_tx.send(x).is_err() {
                    warn!("outin can't send!");
                    break
                }
            }
            warn!("OutputInput is done!");
        });

        Channel { 
            start: start, 
            tx: o_tx,
            rx: i_rx, 
            handle: handle 
        }
    }
}

impl <T> Input<T> for Channel<T> 
where
    T: Send + Clone + 'static
{
    fn source(&self) -> InputSource<T> {
        InputSource { start: self.start.clone(), rx: self.rx.resubscribe() }
    }
}

impl <T> Output<T> for Channel<T> 
where
    T: Send + Clone + 'static
{
    fn sink(&self) -> OutputSink<T> {
        OutputSink { tx: self.tx.clone() }
    }
}


#[derive(Deserialize, Debug)]
pub enum ChannelConfig {
    Float{
        start: f64
    },
    Bool{
        start: bool
    },
    String{
        start: String
    },
}

impl Into<ChannelBox> for ChannelConfig {
    fn into(self) -> ChannelBox {
        match self {
            ChannelConfig::Float { start } => ChannelBox::Float(Channel::new(start)),
            ChannelConfig::Bool{ start } => ChannelBox::Bool(Channel::new(start)),
            ChannelConfig::String { start } => ChannelBox::String(Channel::new(start)),
        }
    }
}