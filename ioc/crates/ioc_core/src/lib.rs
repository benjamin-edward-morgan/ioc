//!This is the core library for the IOC project. All other IOC libraries depend on this one. This includes all fundamental data types required for a running IOC instance.

use error::IocBuildError;
use std::{collections::HashMap, fmt, future::Future};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

pub mod error;
pub mod pipe;
pub mod transformer;
pub mod feedback;

///An input source from an IOC component. Always starts with a value and includes a receiver so the conumer can receive updated values.
pub struct InputSource<T> {
    pub start: T,
    pub rx: broadcast::Receiver<T>,
}

///An input value from an IOC component. Multiple `InputSource`s can be created from a single `Input`.
///
/// `Input`s can take any type, but must be restricted to a fundamental data type when using configuration
pub trait Input<T> {
    //Produces a new InputSource for this Input
    fn source(&self) -> InputSource<T>;
}

///An output sink to an IOC component. A producer can write values to the sender and the component receives the values.
pub struct OutputSink<T> {
    pub tx: mpsc::Sender<T>,
}

///An output to an IOC component. Although they can be created, there should not be multiple `OutputSink`s writing to a single output.
///
/// `Output`s can take any type, but must be restricted to a fundamental data type when using configuration
pub trait Output<T> {
    fn sink(&self) -> OutputSink<T>;
}

///Enum to hold fundamental data type values.
///
/// TODO: add other types this is just used for arrays at the moment
#[derive(Debug, Clone)]
pub enum Value {
    Float(f64),
}

///Fundamental `Input` kinds when using configuration.
pub enum InputKind {
    String(Box<dyn Input<String>>),
    Binary(Box<dyn Input<Vec<u8>>>),
    Float(Box<dyn Input<f64>>),
    Bool(Box<dyn Input<bool>>),
    Array(Box<dyn Input<Vec<Value>>>),
}

///Some functions to save you from writing `Box::new`
impl InputKind {
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
    pub fn array<F: Input<Vec<Value>> + 'static>(f: F) -> Self {
        Self::Array(Box::new(f))
    }
}

impl fmt::Debug for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
            Self::Array(_) => f.write_str("Array"),
        }
    }
}

///Fundamental `Output` kinds when using configuration.
pub enum OutputKind {
    String(Box<dyn Output<String>>),
    Binary(Box<dyn Output<Vec<u8>>>),
    Float(Box<dyn Output<f64>>),
    Bool(Box<dyn Output<bool>>),
    Array(Box<dyn Output<Vec<Value>>>),
}

impl fmt::Debug for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
            Self::Array(_) => f.write_str("Array"),
        }
    }
}

///some functions to save you from writing `Box::new`
impl OutputKind {
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

///When using configuration, ModuleIO holds the inputs, outputs and a join handle provided by a `Module`.
///
///Callers should use `join_handle.await`  
pub struct ModuleIO {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
    pub outputs: HashMap<String, OutputKind>,
}

///A configurable entity that can provide a ModuleIO. A `Module` represents some collection of inputs and/or outputs that can be wired into an IOC graph.
///
/// A module represents some black-box entity that provides zero or more `Input`s and zero or more `Output`s. These inputs and outputs are not necessarily coupled, and could have different semanit meanings.
///
/// Modules could provide access to physical hardware. For example, a module may only provide a single `Input` if it represents a sensor reading. A module that emits a single `Output` could represent a light or an actuator.
///
/// The `ioc_server` crate exposes a `Module`` with `Input`s and `Output`s that may by written and read (respectively) over a web socket connection.
pub trait Module: Into<ModuleIO> {
    type Config;

    fn try_build(cfg: &Self::Config) -> impl Future<Output = Result<Self, IocBuildError>>;
}

///Similar to a `Module`, this is an entity to construct a `Module`. This is useful when there is some
pub trait ModuleBuilder {
    type Config;
    type Module: Into<ModuleIO>;

    fn try_build(
        &self,
        cfg: &Self::Config,
    ) -> impl Future<Output = Result<Self::Module, IocBuildError>>;
}

///When using configuration, `TransformerI` holds the inputs and a join handle provided by a `Transformer`
///Callers should use `join_handle.await`
pub struct TransformerI {
    pub join_handle: JoinHandle<()>,
    pub inputs: HashMap<String, InputKind>,
}

///Similar to `Modules`, but only provides `Input`s. `Transformer`s are typically _constructed_ consuming other inputs.
///
/// All `Transformer`s could be implemented as `Module`s, but this would require many additional `Pipe`s to connect inputs to outputs. A `Transformer` reduces this verbosity.
///
/// `Transformer`s can be thought of as simple functions that consume from one or more `Input`s and provide (one or more) `Input`s that emit the function's outputs.
pub trait Transformer<'a>: Into<TransformerI> {
    type Config;

    fn try_build(cfg: &Self::Config) -> impl Future<Output = Result<Self, IocBuildError>>;
}
