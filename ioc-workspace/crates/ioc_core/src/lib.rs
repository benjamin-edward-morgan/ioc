use std::{collections::HashMap, fmt, future::Future, rc::Rc};
use tokio::{sync::{broadcast, mpsc}, task::JoinHandle};

pub mod channel;
pub mod pipe;
pub mod transformer;

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

pub enum InputKind {
    String(Box<dyn Input<String>>),
    Binary(Box<dyn Input<Vec<u8>>>),
    Float(Box<dyn Input<f64>>),
    Bool(Box<dyn Input<bool>>),
}

impl fmt::Debug for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
        }
    }
}

pub enum OutputKind {
    String(Box<dyn Output<String>>),
    Binary(Box<dyn Output<Vec<u8>>>),
    Float(Box<dyn Output<f64>>),
    Bool(Box<dyn Output<bool>>),
}

impl fmt::Debug for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
        }
    }
}

pub struct ModuleIO {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
    pub outputs: HashMap<String, OutputKind>,
}

pub trait ModuleBuilder: Into<ModuleIO> {
    type Config;
    type Error; 

    fn try_build(cfg: &Self::Config) -> impl Future<Output=Result<Self, Self::Error>> ;
}

pub struct TransformerI {
    pub inputs: HashMap<String, InputKind>,
}

pub trait Transformer<'a>: Into<TransformerI> {
    type Config;
    type Error;

    fn try_build(cfg: &Self::Config) -> impl Future<Output=Result<Self, Self::Error>> ;
}