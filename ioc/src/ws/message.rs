use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};
use serde::{Serialize,Deserialize};

use super::state::{IOUpdate, WsInputStateValue, WsOutputStateValue, WsStateValue};

#[derive(Serialize, Clone, Debug)]
pub struct WsTimestamp{
    seconds: f64
}

impl WsTimestamp {
    pub fn now() -> Self {
        let now = 
            SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("bad time!");
        WsTimestamp { 
            seconds: now.as_secs_f64()
        }
    }
}

#[derive(Serialize,Clone,Debug)]
pub enum WsInputStateInitial {
    Bool{ 
        value: bool
    },
    Float{ 
        value: f64, 
        min: f64,
        max: f64,
        step: f64
    },
    String{
        value: String,
        max_length: usize 
    }
}

impl WsInputStateInitial {
    pub fn from_state(i_state: WsInputStateValue) -> WsInputStateInitial {
        match i_state {
            WsInputStateValue::Bool { b } => 
                WsInputStateInitial::Bool { value: b },
            WsInputStateValue::Float { f, min, max, step } =>
                WsInputStateInitial::Float { value: f, min: min, max: max, step: step },
            WsInputStateValue::String { s, max_length } => 
                WsInputStateInitial::String { value: s, max_length: max_length }
        }
    }
}

#[derive(Serialize,Clone,Debug)]
pub enum WsOutputStateInitial {
    Bool {
        value: Option<bool>
    },
    Float {
        value: Option<f64>
    },
    String{
        value: Option<String>
    }
}

impl WsOutputStateInitial {
    pub fn from_state(o_state: WsOutputStateValue) -> WsOutputStateInitial {
        match o_state {
            WsOutputStateValue::Bool { b } => 
                WsOutputStateInitial::Bool { value: b },
            WsOutputStateValue::Float { f } => 
                WsOutputStateInitial::Float { value: f },
            WsOutputStateValue::String { s } => 
                WsOutputStateInitial::String { value: s },
        }
    }
}

#[derive(Serialize,Clone,Debug)]
pub struct WsInitialMessage{
    inputs: HashMap<String, WsInputStateInitial>,
    outputs: HashMap<String, WsOutputStateInitial>,
    time: WsTimestamp
}

impl WsInitialMessage {
    pub fn from_state(io_update: IOUpdate) -> WsInitialMessage {
        WsInitialMessage { 
            inputs: io_update.inputs.iter().map(|(k, v)| {
                (k.to_string(), WsInputStateInitial::from_state((*v).clone()))
            }).collect(), 
            outputs: io_update.outputs.iter().map(|(k, v)| {
                (k.to_string(), WsOutputStateInitial::from_state((*v).clone()))
            }).collect(),
            time: WsTimestamp::now()
        }
    }
}

#[derive(Serialize,Deserialize,Clone,Debug)]
pub enum WsStateUpdate{
    Bool{ 
        value: bool
    },
    Float{ 
        value: f64, 
    },
    String{
        value: String,
    }
}

impl WsStateUpdate {
    pub fn from_i_state(i_state: WsInputStateValue ) -> WsStateUpdate {
        match i_state {
            WsInputStateValue::Bool { b } => 
                WsStateUpdate::Bool { value: b },
            WsInputStateValue::Float { f, min: _, max: _, step: _ } =>
                WsStateUpdate::Float { value: f },
            WsInputStateValue::String { s, max_length: _ } => 
                WsStateUpdate::String { value: s }
        }
    }
    pub fn from_o_state(o_state: WsOutputStateValue ) -> Option<WsStateUpdate> {
        match o_state {
            WsOutputStateValue::Bool { b: Some(value) } => 
                Some(WsStateUpdate::Bool { value: value }),
            WsOutputStateValue::Float { f: Some(value) } =>
                Some(WsStateUpdate::Float { value: value }),
            WsOutputStateValue::String { s: Some(value) } => 
                Some(WsStateUpdate::String { value: value }),
            _ => None
        }
    }
    pub fn to_state(self) ->  WsStateValue {
        match self {
            WsStateUpdate::Bool { value } => 
                WsStateValue::Bool { b: value },
            WsStateUpdate::Float { value } =>
                WsStateValue::Float { f: value },
            WsStateUpdate::String { value } => 
                WsStateValue::String { s: value },
        }
    }
}

#[derive(Serialize,Clone,Debug)]
pub struct WsUpdateMessage{
    pub inputs: HashMap<String, WsStateUpdate>,
    pub outputs: HashMap<String, WsStateUpdate>,
    pub time: WsTimestamp,
}

impl WsUpdateMessage{
    pub fn from_state(io_update: IOUpdate) -> WsUpdateMessage {
        WsUpdateMessage { 
            inputs: io_update.inputs.iter().map(|(k, v)| {
                (k.to_string(), WsStateUpdate::from_i_state((*v).clone()))
            }).collect(), 
            outputs: io_update.outputs.iter().flat_map(|(k, v)| {
                WsStateUpdate::from_o_state((*v).clone()).map(|state_update| {
                    (k.to_string(), state_update)
                })
            }).collect(),
            time: WsTimestamp::now()
        }
    }
}