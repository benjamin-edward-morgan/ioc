use crate::error::ServerBuildError;
use crate::{ServerInputConfig, ServerOutputConfig};
use std::collections::{HashMap,HashSet};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{info,error};
use std::hash::{Hash,Hasher};

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
        max_length: u32,
    },
}


#[derive(Debug, Clone)]
pub(crate) enum ServerOutputState {
    Float { value: Option<f64> },
    Bool { value: Option<bool> },
    String { value: Option<String> },
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
    Subscribe{callback: oneshot::Sender<Subscription>, inputs: HashSet<String>, outputs: HashSet<String>},
}

impl From<&ServerInputConfig> for ServerInputState {
    fn from(config: &ServerInputConfig) -> Self {
        match config {
            ServerInputConfig::Float{ start, min, max, step } => 
                ServerInputState::Float{  value: *start, min: *min, max: *max, step: *step},
            ServerInputConfig::Bool{ start } => 
                ServerInputState::Bool{ value: *start },
            ServerInputConfig::String{ start, max_length } => 
                ServerInputState::String{ value: start.to_string(), max_length: *max_length }
        }
    }
}

impl From<&ServerOutputConfig> for ServerOutputState {
    fn from(config: &ServerOutputConfig) -> Self {
        match config {
            ServerOutputConfig::Float => ServerOutputState::Float{ value: None},
            ServerOutputConfig::Bool => ServerOutputState::Bool{ value: None},
            ServerOutputConfig::String => ServerOutputState::String{ value: None},     
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
        inputs: &HashMap<&str, ServerInputConfig>,
        outputs: &HashMap<&str, ServerOutputConfig>,
    ) -> Result<Self, ServerBuildError> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel(channel_size);

        let mut internal_inputs = HashMap::with_capacity(inputs.len());
        let mut input_states: HashMap<String,ServerInputState> = HashMap::with_capacity(inputs.len());
        for (key, input) in inputs {
            internal_inputs.insert(key.to_string(), input.into());
            input_states.insert(key.to_string(), input.into());
        }
        
        let mut internal_outputs = HashMap::with_capacity(inputs.len());
        let mut output_states: HashMap<String,ServerOutputState> = HashMap::with_capacity(outputs.len());
        for (key, output) in outputs {
            internal_outputs.insert(key.to_string(), output.into());
            output_states.insert(key.to_string(), output.into());
        }

        let mut state_subs = StateSubscriptions::with_capacities(100, 100);

        let handle = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    StateCmd::Subscribe{ callback, inputs, outputs } => {

                        let subs_rx = state_subs.subscribe(inputs, outputs); 

                        //TODO: filter intial inputs and outputs
                        let subs = Subscription {
                            start: StateUpdate{
                                inputs: internal_inputs.clone(),
                                outputs: internal_outputs.clone(),
                            },
                            update_rx: subs_rx,
                        };

                        if let Err(err) = callback.send(subs) {
                            error!("error sending subscription! {:?}", err)
                        }
                    },
                    StateCmd::Update(update) => { 

                        let mut inputs = HashMap::with_capacity(update.inputs.len());
                        let mut outputs = HashMap::with_capacity(update.inputs.len());

                        for (k, update_i) in update.inputs {
                            if let Some(current_i) = internal_inputs.get_mut(&k) {
                                match (update_i, current_i) {
                                    (ServerInputState::Float{ value: updated_float_value, ..}, ServerInputState::Float{ value: current_float_value, min, max, step }) => {
                                        *current_float_value = updated_float_value;
                                        inputs.insert(k, ServerInputState::Float{ value: updated_float_value, min: *min, max: *max, step: *step });
                                    },
                                    (ServerInputState::Bool{ value: updated_bool_value }, ServerInputState::Bool{ value: current_bool_value }) => {
                                        *current_bool_value = updated_bool_value;
                                        inputs.insert(k, ServerInputState::Bool{ value: updated_bool_value });
                                    },
                                    (ServerInputState::String{ value: updated_string_value, ..}, ServerInputState::String{ value: current_string_value, max_length }) => {
                                        *current_string_value = updated_string_value.to_string();
                                        inputs.insert(k, ServerInputState::String{ value: updated_string_value, max_length: *max_length });
                                    },
                                    (_, _) => panic!("nope!"),
                                }
                            }
                        }

                        for (k, update_o) in update.outputs {
                            if let Some(current_o) = internal_outputs.get_mut(&k) {
                                match (update_o, current_o) {
                                    (ServerOutputState::Float{ value: updated_float_value }, ServerOutputState::Float{ value: current_float_value }) => {
                                        *current_float_value = updated_float_value;
                                        outputs.insert(k, ServerOutputState::Float{ value: updated_float_value });
                                    },
                                    (ServerOutputState::Bool{ value: updated_bool_value }, ServerOutputState::Bool{ value: current_bool_value }) => {
                                        *current_bool_value = updated_bool_value;
                                        outputs.insert(k, ServerOutputState::Bool{ value: updated_bool_value });
                                    },
                                    (ServerOutputState::String{ value: updated_string_value }, ServerOutputState::String{ value: current_string_value }) => {
                                        *current_string_value = updated_string_value.clone();
                                        outputs.insert(k, ServerOutputState::String{ value: updated_string_value });
                                    },
                                    (_, _) => panic!("nope!"),
                                }
                            }
                        }

                        state_subs.publish(
                            StateUpdate{
                                inputs, 
                                outputs,
                            }
                        )
                    },
                }
            }
            info!("ServerState is done!");
        });

        Ok(ServerState { handle, cmd_tx })
    }
}

struct StateSubscription {
    tx: broadcast::Sender<StateUpdate>,
    rx: broadcast::Receiver<StateUpdate>
}

impl StateSubscription {
    pub fn with_channel_size(channel_size: usize) -> Self {
        let (tx, rx) = broadcast::channel(channel_size);
        Self{ rx, tx }
    }
}


#[derive(Eq, PartialEq, Debug)]
struct StateSubscriptionKey {
    inputs: HashSet<String>,
    outputs: HashSet<String>
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
            Some(StateUpdate{
                inputs, 
                outputs,
            })
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
        StateSubscriptions{
            channel_size,
            subscriptions: HashMap::with_capacity(subscriptions)
        }
    }

    pub fn subscribe(&mut self, inputs: HashSet<String>, outputs: HashSet<String>) -> broadcast::Receiver<StateUpdate> {
        let subs_key = StateSubscriptionKey{
            inputs,
            outputs,
        };
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