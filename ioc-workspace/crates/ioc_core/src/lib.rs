use std::{collections::HashMap, fmt, future::Future};
use error::IocBuildError;
use tokio::{sync::{broadcast, mpsc}, task::JoinHandle};

pub mod channel;
pub mod pipe;
pub mod transformer;
pub mod error; 

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

impl InputKind {
    //some functions to save you from writing `Box::new`
    pub fn float<F: Input<f64> + 'static>(f: F) -> Self {
        Self::Float(Box::new(f))
    }
    pub fn binary<F: Input<Vec<u8>> + 'static>(f: F) -> Self {
        Self::Binary(Box::new(f))
    }
    pub fn string<F: Input<String> + 'static>(f: F) -> Self {
        Self::String(Box::new(f))
    }
    pub fn bool<F: Input<bool> + 'static>(f: F) -> Self {
        Self::Bool(Box::new(f))
    }
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

impl OutputKind {
    //some functions to save you from writing `Box::new`
    pub fn float<F: Output<f64> + 'static>(f: F) -> Self {
        Self::Float(Box::new(f))
    }
    pub fn binary<F: Output<Vec<u8>> + 'static>(f: F) -> Self {
        Self::Binary(Box::new(f))
    }
    pub fn string<F: Output<String> + 'static>(f: F) -> Self {
        Self::String(Box::new(f))
    }
    pub fn bool<F: Output<bool> + 'static>(f: F) -> Self {
        Self::Bool(Box::new(f))
    }
}

pub struct ModuleIO {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
    pub outputs: HashMap<String, OutputKind>,
}

pub trait Module: Into<ModuleIO> {
    type Config;

    fn try_build(cfg: &Self::Config) -> impl Future<Output=Result<Self, IocBuildError>> ;
}

pub trait ModuleBuilder {
    type Config;
    type Module: Into<ModuleIO>;

    fn try_build(&self, cfg: &Self::Config) -> impl Future<Output=Result<Self::Module, IocBuildError>> ;
}

pub struct TransformerI {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
}

pub trait Transformer<'a>: Into<TransformerI> {
    type Config;

    fn try_build(cfg: &Self::Config) -> impl Future<Output=Result<Self, IocBuildError>> ;
}