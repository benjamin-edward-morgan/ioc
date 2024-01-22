use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use crate::server::state::{StateUpdate, ServerInputState, ServerOutputState};

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
}

impl From<ServerInputState> for WsInputStateInitial {
    fn from(state: ServerInputState) -> Self {
        match state {
            ServerInputState::Float{value, min, max, step} => 
                WsInputStateInitial::Float{value, min, max, step},
            ServerInputState::Bool{value} => 
                WsInputStateInitial::Bool{value},
            ServerInputState::String{value, max_length} => 
                WsInputStateInitial::String{value, max_length},     
        }
    }
}

#[derive(Serialize)]
pub enum WsOutputStateInitial {
    Float { value: Option<f64> },
    Bool { value: Option<bool> },
    String { value: Option<String> },
}

impl From<ServerOutputState> for WsOutputStateInitial {
    fn from(state: ServerOutputState) -> Self {
        match state {
            ServerOutputState::Float{ value } => 
                WsOutputStateInitial::Float{ value },
            ServerOutputState::Bool{ value } => 
                WsOutputStateInitial::Bool{ value },
            ServerOutputState::String{ value } => 
                WsOutputStateInitial::String{ value },      
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

        Self{
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
}

impl From<WsStateUpdate> for ServerInputState {
    fn from(update: WsStateUpdate) -> Self {
        match update {
            WsStateUpdate::Float{ value } => ServerInputState::Float{ value, min: 0.0, max: 0.0, step: 0.0 },
            WsStateUpdate::Bool{ value } => ServerInputState::Bool{ value },
            WsStateUpdate::String{ value } => ServerInputState::String{ value, max_length: 0 },
        }
    }
}

impl From<HashMap<String, WsStateUpdate>> for StateUpdate {
    fn from(update: HashMap<String, WsStateUpdate>) -> Self {
        
        let mut inputs = HashMap::with_capacity(update.len());
        for (k, i) in update {
            inputs.insert(k, i.into());
        }

        StateUpdate{
            inputs,
            outputs: HashMap::new()
        }
    }
}

#[derive(Serialize)]
pub struct WsUpdateMessage {
    pub inputs: HashMap<String, WsStateUpdate>,
    pub outputs: HashMap<String, WsStateUpdate>,
    pub time: WsTimestamp,
}