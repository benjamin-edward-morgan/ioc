use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::warn;

use super::{WsStateConfig, WsStateInputConfig, WsStateOutputConfig};

#[derive(Clone, Debug)]
pub enum WsStateValue {
    Bool { b: bool },
    Float { f: f64 },
    String { s: String },
}

#[derive(Clone, Debug)]
pub enum WsInputStateValue {
    Bool {
        b: bool,
    },
    Float {
        f: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    String {
        s: String,
        max_length: usize,
    },
}

impl From<&WsStateInputConfig> for WsInputStateValue {
    fn from(value: &WsStateInputConfig) -> Self {
        match value {
            WsStateInputConfig::Bool(cfg) => WsInputStateValue::Bool { b: cfg.start },
            WsStateInputConfig::Float(cfg) => {
                if cfg.min > cfg.max {
                    panic!("min must be <= max");
                }
                WsInputStateValue::Float {
                    f: cfg.start,
                    min: cfg.min,
                    max: cfg.max,
                    step: cfg.step,
                }
            }
            WsStateInputConfig::String(cfg) => WsInputStateValue::String {
                s: cfg.start.to_string(),
                max_length: cfg.max_length,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum WsOutputStateValue {
    Bool { b: Option<bool> },
    Float { f: Option<f64> },
    String { s: Option<String> },
}

impl From<&WsStateOutputConfig> for WsOutputStateValue {
    fn from(value: &WsStateOutputConfig) -> Self {
        match value {
            WsStateOutputConfig::Bool => WsOutputStateValue::Bool { b: None },
            WsStateOutputConfig::Float => WsOutputStateValue::Float { f: None },
            WsStateOutputConfig::String => WsOutputStateValue::String { s: None },
        }
    }
}

#[derive(Clone, Debug)]
pub struct IUpdate {
    pub inputs: HashMap<String, WsInputStateValue>,
}

#[derive(Clone, Debug)]
pub struct IOUpdate {
    pub inputs: HashMap<String, WsInputStateValue>,
    pub outputs: HashMap<String, WsOutputStateValue>,
}

#[derive(Debug)]
pub struct ISubscription {
    pub start: IUpdate,
    pub receiver: broadcast::Receiver<IUpdate>,
}

#[derive(Debug)]
pub struct IOSubscription {
    pub start: IOUpdate,
    pub receiver: broadcast::Receiver<IOUpdate>,
}

#[derive(Debug)]
pub enum WsStateCmd {
    SetInputs {
        state: HashMap<String, WsStateValue>,
    },
    SetOutputs {
        state: HashMap<String, WsStateValue>,
    },
    SubscribeInputs {
        subs_callback: oneshot::Sender<ISubscription>,
    },
    SubscribeAll {
        subs_callback: oneshot::Sender<IOSubscription>,
    },
}

pub struct WsState {
    pub cmd_tx: mpsc::Sender<WsStateCmd>,
    pub handle: JoinHandle<()>,
}

impl WsState {
    pub fn new(config: &WsStateConfig) -> WsState {
        let mut input_state: HashMap<String, WsInputStateValue> =
            HashMap::with_capacity(config.input_configs.len());
        config.input_configs.iter().for_each(|(key, value)| {
            input_state.insert(key.to_string(), value.into());
        });

        let mut output_state: HashMap<String, WsOutputStateValue> =
            HashMap::with_capacity(config.output_configs.len());
        config.output_configs.iter().for_each(|(key, value)| {
            output_state.insert(key.to_string(), value.into());
        });

        let (i_tx, i_rx) = broadcast::channel(config.channel_size);
        let (io_tx, io_rx) = broadcast::channel(config.channel_size);

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsStateCmd>(10);

        let handle = tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let result = match cmd {
                    WsStateCmd::SubscribeInputs { subs_callback } => {
                        let subs = ISubscription {
                            start: IUpdate {
                                inputs: input_state.clone(),
                            },
                            receiver: i_rx.resubscribe(),
                        };
                        subs_callback.send(subs).map_err(|err| {
                            format!("Error sending ISubscription to callback: {:?}", err)
                        })
                    }
                    WsStateCmd::SubscribeAll { subs_callback } => {
                        let subs = IOSubscription {
                            start: IOUpdate {
                                inputs: input_state.clone(),
                                outputs: output_state.clone(),
                            },
                            receiver: io_rx.resubscribe(),
                        };
                        subs_callback.send(subs).map_err(|err| {
                            format!("Error sending IOSubscription to callback {:?}", err)
                        })
                    }
                    WsStateCmd::SetInputs { state } => {
                        let mut updates: HashMap<String, WsInputStateValue> =
                            HashMap::with_capacity(state.len());

                        state.iter().for_each(|(key, new_value)| {
                            match (input_state.get_mut(key), new_value) {
                                (
                                    Some(WsInputStateValue::Bool { b: current }),
                                    WsStateValue::Bool { b: new },
                                ) => {
                                    if *current != *new {
                                        *current = *new;
                                        updates.insert(
                                            key.to_string(),
                                            WsInputStateValue::Bool { b: *new },
                                        );
                                    }
                                }

                                (
                                    Some(WsInputStateValue::Float {
                                        f: current,
                                        min,
                                        max,
                                        step,
                                    }),
                                    WsStateValue::Float { f: new },
                                ) => {
                                    let clamped = (*new).min(*max).max(*min);
                                    if *current != clamped {
                                        *current = clamped;
                                        updates.insert(
                                            key.to_string(),
                                            WsInputStateValue::Float {
                                                f: clamped,
                                                min: *min,
                                                max: *max,
                                                step: *step,
                                            },
                                        );
                                    }
                                }

                                (
                                    Some(WsInputStateValue::String {
                                        s: current,
                                        max_length,
                                    }),
                                    WsStateValue::String { s: new },
                                ) => {
                                    let trimed: String =
                                        new.to_string().chars().take(*max_length).collect();
                                    if !current.to_string().eq(&trimed) {
                                        *current = trimed.clone();
                                        updates.insert(
                                            key.to_string(),
                                            WsInputStateValue::String {
                                                s: trimed,
                                                max_length: *max_length,
                                            },
                                        );
                                    }
                                }

                                (Some(_current), _new) => { /* incompatible type */ }
                                (None, _) => { /* non existant key */ }
                            }
                        });

                        if !updates.is_empty() {
                            let i_result = i_tx
                                .send(IUpdate {
                                    inputs: updates.clone(),
                                })
                                .map_err(|err| format!("Error sending IUpdate! {:?}", err));

                            let io_result = io_tx
                                .send(IOUpdate {
                                    inputs: updates,
                                    outputs: HashMap::new(),
                                })
                                .map_err(|err| format!("Error sending IOUpdate! {:?}", err));

                            match (i_result, io_result) {
                                (Ok(_), Ok(_)) => Ok(()),
                                (Err(i_err), Ok(_)) => Err(i_err),
                                (Ok(_), Err(io_err)) => Err(io_err),
                                (Err(i_err), Err(io_err)) => {
                                    Err(format!("i error: {}\nio error: {}", i_err, io_err))
                                }
                            }
                        } else {
                            //no updates to send (set to same value)
                            Ok(())
                        }
                    }

                    WsStateCmd::SetOutputs { state } => {
                        let mut updates: HashMap<String, WsOutputStateValue> =
                            HashMap::with_capacity(state.len());

                        state.iter().for_each(|(key, value)| {
                            match (output_state.get_mut(key), value) {
                                (
                                    Some(WsOutputStateValue::Bool { b: current }),
                                    WsStateValue::Bool { b: new },
                                ) => match current {
                                    Some(current) => {
                                        if current != new {
                                            *current = *new;
                                            updates.insert(
                                                key.to_string(),
                                                WsOutputStateValue::Bool { b: Some(*new) },
                                            );
                                        }
                                    }
                                    None => {
                                        *current = Some(*new);
                                        updates.insert(
                                            key.to_string(),
                                            WsOutputStateValue::Bool { b: Some(*new) },
                                        );
                                    }
                                },

                                (
                                    Some(WsOutputStateValue::Float { f: current }),
                                    WsStateValue::Float { f: new },
                                ) => match current {
                                    Some(current) => {
                                        if current != new {
                                            *current = *new;
                                            updates.insert(
                                                key.to_string(),
                                                WsOutputStateValue::Float { f: Some(*new) },
                                            );
                                        }
                                    }
                                    None => {
                                        *current = Some(*new);
                                        updates.insert(
                                            key.to_string(),
                                            WsOutputStateValue::Float { f: Some(*new) },
                                        );
                                    }
                                },

                                (
                                    Some(WsOutputStateValue::String { s: current }),
                                    WsStateValue::String { s: new },
                                ) => match current {
                                    Some(current) => {
                                        if current != new {
                                            *current = (*new).clone();
                                            updates.insert(
                                                key.to_string(),
                                                WsOutputStateValue::String {
                                                    s: Some((*new).clone()),
                                                },
                                            );
                                        }
                                    }
                                    None => {
                                        *current = Some((*new).clone());
                                        updates.insert(
                                            key.to_string(),
                                            WsOutputStateValue::String {
                                                s: Some((*new).clone()),
                                            },
                                        );
                                    }
                                },

                                (Some(_current), _new) => { /* incompatible type */ }
                                (None, _) => { /* non existant key */ }
                            }
                        });

                        if !updates.is_empty() {
                            io_tx
                                .send(IOUpdate {
                                    inputs: HashMap::new(),
                                    outputs: updates,
                                })
                                .map_err(|err| format!("Error sending IOUpdate! {:?}", err))
                                .map(|_| ())
                        } else {
                            //no updates to send (set to same value)
                            Ok(())
                        }
                    }
                };

                if let Err(err_str) = result {
                    warn!("Error in WSState!!! :(\n{:?}", err_str);
                }
            }
            warn!("ws state is done!");
        });

        WsState { cmd_tx, handle }
    }
}
