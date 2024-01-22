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
        for (key, input) in inputs {
            internal_inputs.insert(key.to_string(), input.into());
        }
        
        let mut internal_outputs = HashMap::with_capacity(inputs.len());
        for (key, output) in outputs {
            internal_outputs.insert(key.to_string(), output.into());
        }

        let mut state_subs = StateSubscriptions::with_capacities(100, 100);

        let handle = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    StateCmd::Subscribe{ callback, inputs, outputs } => {

                        let subs_rx = state_subs.subscribe(inputs, outputs); 

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

                        //todo: actually _save_ that updates
                        state_subs.publish(update)
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


#[derive(Eq, PartialEq)]
struct StateSubscriptionKey {
    inputs: HashSet<String>,
    outputs: HashSet<String>
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
            inputs: inputs,
            outputs: outputs,
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
        todo!();
    }
}