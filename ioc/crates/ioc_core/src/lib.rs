//!This is the core library for the IOC project. All other IOC libraries depend on this one. This includes all fundamental data types required for a running IOC instance.

use error::IocBuildError;
use serde::{de::Visitor, ser::SerializeSeq, ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, fmt, future::Future};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub mod error;
pub mod pipe;
pub mod transformer;
pub mod feedback;

pub struct Input<T>{
    rx: watch::Receiver<T>
}

impl<T> Input<T> {
    pub fn new(start: T) -> (Self, watch::Sender<T>) {
        let (tx, rx) = watch::channel(start);
        (Self { rx }, tx)
    }
    pub fn source(&self) -> watch::Receiver<T> {
        self.rx.clone()
    }
}

pub struct Output<T>{
    pub tx: mpsc::Sender<T>
}

impl<T> Output<T> {
    pub fn new() -> (Self, mpsc::Receiver<T>) {
        let (tx, rx) = mpsc::channel(1);
        (Self { tx }, rx)
    }
    pub fn sink(&self) -> mpsc::Sender<T> {
        self.tx.clone()
    }
}

///Enum to hold fundamental data type values.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Binary(Vec<u8>),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor; 

impl <'a> Visitor<'a> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Value::String(v.to_string()))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(Value::Binary(v.to_vec()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'a>,
    {
        let mut vec = Vec::new();
        while let Some(value) = seq.next_element()? {
            vec.push(value);
        }
        Ok(Value::Array(vec))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'a>,
    {
        let mut obj = HashMap::new();
        while let Some((key, value)) = map.next_entry()? {
            obj.insert(key, value);
        }
        Ok(Value::Object(obj))
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::String(s) => ser.serialize_str(s),
            Value::Binary(b) => ser.serialize_bytes(b),
            Value::Float(f) => ser.serialize_f64(*f),
            Value::Bool(b) => ser.serialize_bool(*b),
            Value::Array(a) => {
                let mut seq = ser.serialize_seq(Some(a.len()))?;
                for v in a {
                    seq.serialize_element(v)?;
                }
                seq.end()
            },
            Value::Object(o) => {
                let mut obj = ser.serialize_map(Some(o.len()))?;
                for (k, v) in o {
                    obj.serialize_entry(k, v)?;
                }
                obj.end()
            },
        }
    }
}

///Fundamental `Input` kinds when using configuration.
pub enum InputKind {
    String(Input<String>),
    Binary(Input<Vec<u8>>),
    Float(Input<f64>),
    Bool(Input<bool>),
    Array(Input<Vec<Value>>),
    Object(Input<HashMap<String, Value>>),
}

impl fmt::Debug for InputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
            Self::Array(_) => f.write_str("Array"),
            Self::Object(_) => f.write_str("Object")
        }
    }
}

///Fundamental `Output` kinds when using configuration.
pub enum OutputKind {
    String(Output<String>),
    Binary(Output<Vec<u8>>),
    Float(Output<f64>),
    Bool(Output<bool>),
    Array(Output<Vec<Value>>),
    Object(Output<HashMap<String, Value>>),
}

impl fmt::Debug for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(_) => f.write_str("String"),
            Self::Binary(_) => f.write_str("Binary"),
            Self::Float(_) => f.write_str("Float"),
            Self::Bool(_) => f.write_str("Bool"),
            Self::Array(_) => f.write_str("Array"),
            Self::Object(_) => f.write_str("Object")
        }
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

    fn try_build(cfg: &Self::Config, cancel_token: CancellationToken) -> impl Future<Output = Result<Self, IocBuildError>>;
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
