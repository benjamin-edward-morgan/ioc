use crate::server::state::{ServerInputState, ServerOutputState, StateUpdate};
use ioc_core::Value;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
pub struct WsTimestamp {
    seconds: f64,
}

impl WsTimestamp {
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("bad time!");
        WsTimestamp {
            seconds: now.as_secs_f64(),
        }
    }
}

#[derive(Serialize)]
pub enum WsInputStateInitial {
    Float {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
        max_length: u32,
    },
    Binary {
        value: Vec<u8>,
    },
    Array {
        value: Vec<f64>,
    },
}

impl From<ServerInputState> for WsInputStateInitial {
    fn from(state: ServerInputState) -> Self {
        match state {
            ServerInputState::Float {
                value,
                min,
                max,
                step,
            } => WsInputStateInitial::Float {
                value,
                min,
                max,
                step,
            },
            ServerInputState::Bool { value } => WsInputStateInitial::Bool { value },
            ServerInputState::String { value, max_length } => {
                WsInputStateInitial::String { value, max_length }
            }
            ServerInputState::Binary { value } => WsInputStateInitial::Binary { value },
            ServerInputState::Array { value } => WsInputStateInitial::Array { value },
        }
    }
}

#[derive(Serialize)]
pub enum WsOutputStateInitial {
    Float { value: Option<f64> },
    Bool { value: Option<bool> },
    String { value: Option<String> },
    Binary { value: Option<Vec<u8>> },
    Array { value: Option<Vec<f64>> },
}

impl From<ServerOutputState> for WsOutputStateInitial {
    fn from(state: ServerOutputState) -> Self {
        match state {
            ServerOutputState::Float { value } => WsOutputStateInitial::Float { value },
            ServerOutputState::Bool { value } => WsOutputStateInitial::Bool { value },
            ServerOutputState::String { value } => WsOutputStateInitial::String { value },
            ServerOutputState::Binary { value } => WsOutputStateInitial::Binary { value },
            ServerOutputState::Array { value } => {
                let value = match value {
                    Some(value_vec) => {
                        let vect = value_vec
                            .iter()
                            .map(|val| match val {
                                Value::Float(f) => *f,
                            })
                            .collect();
                        Some(vect)
                    }
                    None => None,
                };
                WsOutputStateInitial::Array { value }
            }
        }
    }
}

#[derive(Serialize)]
pub struct WsInitialMessage {
    inputs: HashMap<String, WsInputStateInitial>,
    outputs: HashMap<String, WsOutputStateInitial>,
    time: WsTimestamp,
}

impl From<StateUpdate> for WsInitialMessage {
    fn from(state: StateUpdate) -> Self {
        let mut inputs = HashMap::with_capacity(state.inputs.len());
        for (k, i) in state.inputs {
            inputs.insert(k, i.into());
        }

        let mut outputs = HashMap::with_capacity(state.outputs.len());
        for (k, i) in state.outputs {
            outputs.insert(k, i.into());
        }

        let time = WsTimestamp::now();

        Self {
            inputs,
            outputs,
            time,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum WsStateUpdate {
    Bool { value: bool },
    Float { value: f64 },
    String { value: String },
    Binary { value: Vec<u8> },
    Array { value: Vec<f64> },
}

impl From<WsStateUpdate> for ServerInputState {
    fn from(update: WsStateUpdate) -> Self {
        match update {
            WsStateUpdate::Float { value } => ServerInputState::Float {
                value,
                min: 0.0,
                max: 0.0,
                step: 0.0,
            },
            WsStateUpdate::Bool { value } => ServerInputState::Bool { value },
            WsStateUpdate::String { value } => ServerInputState::String {
                value,
                max_length: 0,
            },
            WsStateUpdate::Binary { value } => ServerInputState::Binary { value },
            WsStateUpdate::Array { value } => ServerInputState::Array { value },
        }
    }
}

impl From<ServerInputState> for WsStateUpdate {
    fn from(state: ServerInputState) -> Self {
        match state {
            ServerInputState::Float { value, .. } => WsStateUpdate::Float { value },
            ServerInputState::Bool { value } => WsStateUpdate::Bool { value },
            ServerInputState::String { value, .. } => WsStateUpdate::String { value },
            ServerInputState::Binary { value } => WsStateUpdate::Binary { value },
            ServerInputState::Array { value } => WsStateUpdate::Array { value },
        }
    }
}

impl From<ServerOutputState> for Option<WsStateUpdate> {
    fn from(state: ServerOutputState) -> Self {
        match state {
            ServerOutputState::Float { value, .. } => {
                value.map(|value| WsStateUpdate::Float { value })
            }
            ServerOutputState::Bool { value } => value.map(|value| WsStateUpdate::Bool { value }),
            ServerOutputState::String { value, .. } => value.map(|value| WsStateUpdate::String {
                value: value.to_string(),
            }),
            ServerOutputState::Binary { value } => {
                value.map(|value| WsStateUpdate::Binary { value })
            }
            ServerOutputState::Array { value } => value.map(|value| {
                let value = value
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => *f,
                    })
                    .collect();
                WsStateUpdate::Array { value }
            }),
        }
    }
}

impl From<HashMap<String, WsStateUpdate>> for StateUpdate {
    fn from(update: HashMap<String, WsStateUpdate>) -> Self {
        let mut inputs = HashMap::with_capacity(update.len());
        for (k, i) in update {
            inputs.insert(k, i.into());
        }

        StateUpdate {
            inputs,
            outputs: HashMap::new(),
        }
    }
}

#[derive(Serialize)]
pub struct WsUpdateMessage {
    pub inputs: HashMap<String, WsStateUpdate>,
    pub outputs: HashMap<String, WsStateUpdate>,
    pub time: WsTimestamp,
}

impl From<StateUpdate> for WsUpdateMessage {
    fn from(update: StateUpdate) -> Self {
        let mut inputs = HashMap::with_capacity(update.inputs.len());
        for (k, i) in update.inputs {
            inputs.insert(k, i.into());
        }

        let mut outputs = HashMap::with_capacity(update.outputs.len());
        for (k, o) in update.outputs {
            if let Some(o) = o.into() {
                outputs.insert(k, o);
            }
        }

        let time = WsTimestamp::now();

        Self {
            inputs,
            outputs,
            time,
        }
    }
}