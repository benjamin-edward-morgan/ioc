use crate::{ServerInputConfig, ServerOutputConfig};
use ioc_core::error::IocBuildError;
use ioc_core::Value;
use tokio_util::sync::CancellationToken;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub(crate) enum ServerInputState {
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
        max_length: usize,
    },
    Binary {
        value: Vec<u8>,
    },
    Array {
        value: Vec<Value>,
    },
    Object {
        value: HashMap<String, Value>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum ServerOutputState {
    Float { value: Option<f64> },
    Bool { value: Option<bool> },
    String { value: Option<String> },
    Binary { value: Option<Vec<u8>> },
    Array { value: Option<Vec<Value>> },
    Object { value: Option<HashMap<String, Value>> },
}

#[derive(Debug, Clone)]
pub(crate) struct StateUpdate {
    pub inputs: HashMap<String, ServerInputState>,
    pub outputs: HashMap<String, ServerOutputState>,
}

#[derive(Debug)]
pub(crate) struct Subscription {
    pub start: StateUpdate,
    pub update_rx: broadcast::Receiver<StateUpdate>,
}

#[derive(Debug)]
pub(crate) enum StateCmd {
    Update(StateUpdate),
    Subscribe {
        callback: oneshot::Sender<Subscription>,
        inputs: HashSet<String>,
        outputs: HashSet<String>,
    },
}

impl From<&ServerInputConfig> for ServerInputState {
    fn from(config: &ServerInputConfig) -> Self {
        match config {
            ServerInputConfig::Float { start, min, max, step, } => 
                ServerInputState::Float {
                    value: *start,
                    min: *min,
                    max: *max,
                    step: *step,
                },
            ServerInputConfig::Bool { start } => 
                ServerInputState::Bool { value: *start },
            ServerInputConfig::String { start, max_length } => 
                ServerInputState::String {
                    value: start.to_string(),
                    max_length: *max_length,
                },
            ServerInputConfig::Array { start } => 
                ServerInputState::Array { value: start.clone() },
            ServerInputConfig::Binary { start } => 
                ServerInputState::Binary { value: start.clone() },
            ServerInputConfig::Object { start } =>
                ServerInputState::Object { value: start.clone() },
        }
    }
}

impl From<&ServerOutputConfig> for ServerOutputState {
    fn from(config: &ServerOutputConfig) -> Self {
        match config {
            ServerOutputConfig::Float => ServerOutputState::Float { value: None },
            ServerOutputConfig::Bool => ServerOutputState::Bool { value: None },
            ServerOutputConfig::String => ServerOutputState::String { value: None },
            ServerOutputConfig::Binary => ServerOutputState::Binary { value: None },
            ServerOutputConfig::Array => ServerOutputState::Array { value: None },
            ServerOutputConfig::Object => ServerOutputState::Object { value: None },
        }
    }
}

pub(crate) struct ServerState {
    pub handle: JoinHandle<()>,
    pub cmd_tx: mpsc::Sender<StateCmd>,
}

impl ServerState {
    pub(crate) fn try_build(
        channel_size: usize,
        inputs: &HashMap<String, ServerInputConfig>,
        outputs: &HashMap<String, ServerOutputConfig>,
        cancel_token: CancellationToken,
    ) -> Result<Self, IocBuildError> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel(channel_size);

        let mut internal_inputs: HashMap<String, ServerInputState> =
            HashMap::with_capacity(inputs.len());
        let mut input_states: HashMap<String, ServerInputState> =
            HashMap::with_capacity(inputs.len());
        for (key, input) in inputs {
            internal_inputs.insert(key.to_string(), input.into());
            input_states.insert(key.to_string(), input.into());
        }

        let mut internal_outputs: HashMap<String, ServerOutputState> =
            HashMap::with_capacity(inputs.len());
        let mut output_states: HashMap<String, ServerOutputState> =
            HashMap::with_capacity(outputs.len());
        for (key, output) in outputs {
            internal_outputs.insert(key.to_string(), output.into());
            output_states.insert(key.to_string(), output.into());
        }

        let mut state_subs = StateSubscriptions::with_capacities(100, 100);

        let server_state_handle = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    StateCmd::Subscribe {
                        callback,
                        inputs,
                        outputs,
                    } => {
                        let subs_rx = state_subs.subscribe(inputs.clone(), outputs.clone());

                        let mut filtered_inputs: HashMap<String, ServerInputState> =
                            HashMap::with_capacity(inputs.len());
                        let mut filtered_outputs: HashMap<String, ServerOutputState> =
                            HashMap::with_capacity(outputs.len());

                        for k in inputs {
                            if let Some(i) = internal_inputs.get(&k) {
                                filtered_inputs.insert(k, i.clone());
                            }
                        }

                        for k in outputs {
                            if let Some(o) = internal_outputs.get(&k) {
                                filtered_outputs.insert(k, o.clone());
                            }
                        }

                        let subs = Subscription {
                            start: StateUpdate {
                                inputs: filtered_inputs,
                                outputs: filtered_outputs,
                            },
                            update_rx: subs_rx,
                        };

                        if let Err(err) = callback.send(subs) {
                            error!("error sending subscription! {:?}", err)
                        }
                    }
                    StateCmd::Update(update) => {
                        let mut inputs: HashMap<String, ServerInputState> =
                            HashMap::with_capacity(update.inputs.len());
                        let mut outputs: HashMap<String, ServerOutputState> =
                            HashMap::with_capacity(update.inputs.len());

                        for (k, update_i) in update.inputs {
                            if let Some(current_i) = internal_inputs.get_mut(&k) {
                                match (update_i, current_i) {
                                    (
                                        ServerInputState::Float {
                                            value: updated_float_value,
                                            ..
                                        },
                                        ServerInputState::Float {
                                            value: current_float_value,
                                            min,
                                            max,
                                            step,
                                        },
                                    ) => {
                                        if *current_float_value != updated_float_value {
                                            *current_float_value = updated_float_value;
                                            inputs.insert(
                                                k,
                                                ServerInputState::Float {
                                                    value: updated_float_value,
                                                    min: *min,
                                                    max: *max,
                                                    step: *step,
                                                },
                                            );
                                        }
                                    }
                                    (
                                        ServerInputState::Bool {
                                            value: updated_bool_value,
                                        },
                                        ServerInputState::Bool {
                                            value: current_bool_value,
                                        },
                                    ) => {
                                        if *current_bool_value != updated_bool_value {
                                            *current_bool_value = updated_bool_value;
                                            inputs.insert(
                                                k,
                                                ServerInputState::Bool {
                                                    value: updated_bool_value,
                                                },
                                            );
                                        }
                                    }
                                    (
                                        ServerInputState::String {
                                            value: updated_string_value,
                                            ..
                                        },
                                        ServerInputState::String {
                                            value: current_string_value,
                                            max_length,
                                        },
                                    ) => {
                                        if *current_string_value != updated_string_value {
                                            *current_string_value =
                                                updated_string_value.to_string();
                                            inputs.insert(
                                                k,
                                                ServerInputState::String {
                                                    value: updated_string_value,
                                                    max_length: *max_length,
                                                },
                                            );
                                        }
                                    }
                                    (
                                        ServerInputState::Binary {
                                            value: updated_binary_value,
                                            ..
                                        },
                                        ServerInputState::Binary {
                                            value: current_binary_value,
                                        },
                                    ) => {
                                        if *current_binary_value != updated_binary_value {
                                            *current_binary_value = updated_binary_value.clone();
                                            inputs.insert(
                                                k,
                                                ServerInputState::Binary {
                                                    value: updated_binary_value,
                                                },
                                            );
                                        }
                                    }
                                    (
                                        ServerInputState::Array {
                                            value: updated_array_value,
                                        },
                                        ServerInputState::Array {
                                            value: current_array_value,
                                        },
                                    ) => {
                                        if *current_array_value != updated_array_value {
                                            *current_array_value = updated_array_value.clone();
                                            inputs.insert(
                                                k,
                                                ServerInputState::Array {
                                                    value: updated_array_value,
                                                },
                                            );
                                        }
                                    }
                                    (
                                        ServerInputState::Object {
                                            value: updated_object_value,
                                        },
                                        ServerInputState::Object {
                                            value: current_object_value,
                                        },
                                    ) => {
                                        if *current_object_value != updated_object_value {
                                            *current_object_value = updated_object_value.clone();
                                            inputs.insert(
                                                k,
                                                ServerInputState::Object {
                                                    value: updated_object_value,
                                                },
                                            );
                                        }
                                    }
                                    (_ , _) => panic!("got either mismatched Input types, which shouldn't happen, or someone added a new type but forgot to add a match case in server state.")
                                }
                            }
                        }

                        for (k, update_o) in update.outputs {
                            if let Some(current_o) = internal_outputs.get_mut(&k) {
                                match (update_o, current_o) {
                                    (
                                        ServerOutputState::Float {
                                            value: updated_float_value,
                                        },
                                        ServerOutputState::Float {
                                            value: current_float_value,
                                        },
                                    ) => {
                                        *current_float_value = updated_float_value;
                                        outputs.insert(
                                            k,
                                            ServerOutputState::Float {
                                                value: updated_float_value,
                                            },
                                        );
                                    }
                                    (
                                        ServerOutputState::Bool {
                                            value: updated_bool_value,
                                        },
                                        ServerOutputState::Bool {
                                            value: current_bool_value,
                                        },
                                    ) => {
                                        *current_bool_value = updated_bool_value;
                                        outputs.insert(
                                            k,
                                            ServerOutputState::Bool {
                                                value: updated_bool_value,
                                            },
                                        );
                                    }
                                    (
                                        ServerOutputState::String {
                                            value: updated_string_value,
                                        },
                                        ServerOutputState::String {
                                            value: current_string_value,
                                        },
                                    ) => {
                                        *current_string_value = updated_string_value.clone();
                                        outputs.insert(
                                            k,
                                            ServerOutputState::String {
                                                value: updated_string_value,
                                            },
                                        );
                                    }
                                    (
                                        ServerOutputState::Binary {
                                            value: updated_binary_value,
                                        },
                                        ServerOutputState::Binary {
                                            value: current_binary_value,
                                        },
                                    ) => {
                                        *current_binary_value = updated_binary_value.clone();
                                        outputs.insert(
                                            k,
                                            ServerOutputState::Binary {
                                                value: updated_binary_value,
                                            },
                                        );
                                    }
                                    (
                                        ServerOutputState::Array {
                                            value: updated_array_value,
                                        },
                                        ServerOutputState::Array {
                                            value: current_array_value,
                                        },
                                    ) => {
                                        *current_array_value = updated_array_value.clone();
                                        outputs.insert(
                                            k,
                                            ServerOutputState::Array {
                                                value: updated_array_value,
                                            },
                                        );
                                    }
                                    (
                                        ServerOutputState::Object {
                                            value: updated_object_value,
                                        },
                                        ServerOutputState::Object {
                                            value: current_object_value,
                                        },
                                    ) => {
                                        *current_object_value = updated_object_value.clone();
                                        outputs.insert(
                                            k,
                                            ServerOutputState::Object {
                                                value: updated_object_value,
                                            },
                                        );
                                    },
                                    (_, _) => panic!("got either mismatched Output types, which shouldn't happen, or someone added a new type but forgot to add a match case in server state."),
                                }
                            }
                        }

                        state_subs.publish(StateUpdate { inputs, outputs })
                    }
                }
            }
            debug!("ServerState is done!");
        });

        let handle = tokio::spawn(async move {
            cancel_token.cancelled().await;
            debug!("shutting down ServerState task!");
            server_state_handle.abort();
        });

        Ok(ServerState { handle, cmd_tx })
    }
}

struct StateSubscription {
    tx: broadcast::Sender<StateUpdate>,
    rx: broadcast::Receiver<StateUpdate>,
}

impl StateSubscription {
    pub fn with_channel_size(channel_size: usize) -> Self {
        let (tx, rx) = broadcast::channel(channel_size);
        Self { rx, tx }
    }
}

#[derive(Eq, PartialEq, Debug)]
struct StateSubscriptionKey {
    inputs: HashSet<String>,
    outputs: HashSet<String>,
}

impl StateSubscriptionKey {
    pub fn filter_state_update(&self, update: &StateUpdate) -> Option<StateUpdate> {
        let update_inputs: HashSet<&String> = HashSet::from_iter(update.inputs.keys());
        let subs_inputs: HashSet<&String> = HashSet::from_iter(&self.inputs);
        let input_intersection = update_inputs.intersection(&subs_inputs);

        let update_outputs: HashSet<&String> = HashSet::from_iter(update.outputs.keys());
        let subs_outputs: HashSet<&String> = HashSet::from_iter(&self.outputs);
        let output_intersection = update_outputs.intersection(&subs_outputs);

        let mut inputs = HashMap::with_capacity(subs_inputs.len());
        let mut outputs = HashMap::with_capacity(subs_outputs.len());

        for k in input_intersection {
            if let Some(i) = update.inputs.get(k.as_str()) {
                inputs.insert(k.to_string(), i.clone());
            }
        }
        for k in output_intersection {
            if let Some(o) = update.outputs.get(k.as_str()) {
                outputs.insert(k.to_string(), o.clone());
            }
        }

        if !outputs.is_empty() || !inputs.is_empty() {
            Some(StateUpdate { inputs, outputs })
        } else {
            None
        }
    }
}

const INPUTS_STR: &str = "inputs";
const OUTPUTS_STR: &str = "outputs";

impl Hash for StateSubscriptionKey {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        INPUTS_STR.hash(hasher);
        let mut inputs_vec: Vec<&String> = self.inputs.iter().collect();
        inputs_vec.sort();
        inputs_vec.iter().for_each(|i| {
            i.hash(hasher);
        });
        OUTPUTS_STR.hash(hasher);
        let mut outputs_vec: Vec<&String> = self.outputs.iter().collect();
        outputs_vec.sort();
        outputs_vec.iter().for_each(|o| {
            o.hash(hasher);
        });
    }
}

struct StateSubscriptions {
    channel_size: usize,
    subscriptions: HashMap<StateSubscriptionKey, StateSubscription>,
}

impl StateSubscriptions {
    pub fn with_capacities(subscriptions: usize, channel_size: usize) -> Self {
        StateSubscriptions {
            channel_size,
            subscriptions: HashMap::with_capacity(subscriptions),
        }
    }

    pub fn subscribe(
        &mut self,
        inputs: HashSet<String>,
        outputs: HashSet<String>,
    ) -> broadcast::Receiver<StateUpdate> {
        let subs_key = StateSubscriptionKey { inputs, outputs };
        match self.subscriptions.get(&subs_key) {
            Some(subs) => subs.rx.resubscribe(),
            None => {
                let subs = StateSubscription::with_channel_size(self.channel_size);
                let rx = subs.rx.resubscribe();
                self.subscriptions.insert(subs_key, subs);
                rx
            }
        }
    }

    pub fn publish(&mut self, update: StateUpdate) {
        self.subscriptions.iter().for_each(|(k, subs)| {
            if let Some(filtered) = k.filter_state_update(&update) {
                subs.tx.send(filtered).unwrap();
            }
        });
    }
}
