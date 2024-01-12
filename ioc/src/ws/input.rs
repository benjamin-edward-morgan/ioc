use tokio::sync::{broadcast,watch};
use tracing::warn;
use std::{fmt::Debug, collections::HashMap};
use crate::{Input, InputSource};

use super::state::{ISubscription, WsInputStateValue};

#[derive(Debug)]
pub enum WsInput {
    Bool{ input: WsGenInput<bool> },
    Float{ input: WsGenInput<f64> },
    String{ input: WsGenInput<String> },
}

enum WsInputTx {
    Bool{ tx: watch::Sender<bool> },
    Float{ tx: watch::Sender<f64> },
    String{ tx: watch::Sender<String> },
}

impl WsInput {
    pub fn from_subscription(subs: ISubscription) -> HashMap<String, WsInput> {
        let cap = subs.start.inputs.len();

        //create WsInputs and corresponding Txs
        let mut inputs = HashMap::with_capacity(cap);
        let mut input_txs = HashMap::with_capacity(cap);
        subs.start.inputs.iter().for_each(|(k, v)| {
            match v.clone() {
                WsInputStateValue::Bool { b } => {
                    let (tx, rx) = watch::channel(b);
                    let input = WsInput::Bool { input: WsGenInput::new(rx) };
                    inputs.insert(k.to_string(), input);
                    let input_tx = WsInputTx::Bool { tx: tx };
                    input_txs.insert(k.to_string(), input_tx);
                },
                WsInputStateValue::Float { f, .. } => {
                    let (tx, rx) = watch::channel(f);
                    let input = WsInput::Float { input: WsGenInput::new(rx) };
                    inputs.insert(k.to_string(), input);
                    let input_tx = WsInputTx::Float { tx: tx };
                    input_txs.insert(k.to_string(), input_tx);
                },
                WsInputStateValue::String { s, .. } => {
                    let (tx, rx) = watch::channel(s);
                    let input = WsInput::String { input: WsGenInput::new(rx) };
                    inputs.insert(k.to_string(), input);
                    let input_tx = WsInputTx::String{ tx: tx };
                    input_txs.insert(k.to_string(), input_tx);
                },
            }
        });

        //spawn a task that sends updates to individual WsInputs 
        let mut subs_receiver = subs.receiver;
        tokio::spawn(async move {
            while let Ok(update) = subs_receiver.recv().await {
                update.inputs.iter().for_each(|(k,input_state)| {
                    let input_tx = input_txs.get(k);
                    let result = match (input_tx, input_state) {
                        (
                            Some(WsInputTx::Bool { tx }),
                            WsInputStateValue::Bool { b }
                        ) => {
                            tx.send(*b).map_err(|_| "error sending Bool input update")
                        },
                        (
                            Some(WsInputTx::Float { tx }),
                            WsInputStateValue::Float { f, .. }
                        ) => {
                            tx.send(*f).map_err(|_| "error sending Float input update")
                        },
                        (
                            Some(WsInputTx::String { tx }),
                            WsInputStateValue::String { s, .. }
                        ) => {
                            tx.send(s.to_string()).map_err(|_| "error sending String input update")
                        },
                        (_, _) => {
                            /* noop  */
                            Ok(())
                        }
                    };

                    if let Err(err_msg) = result {
                        warn!("{}", err_msg);
                    }
                });
            }
            warn!("done writing states to ws inputs!");
        });

        inputs
    }
}

#[derive(Debug)]
pub struct WsGenInput<T> {
    current_value: T,
    rx: broadcast::Receiver<T>
}

impl <T> WsGenInput<T> 
where
T: Clone + Debug + Sync + Send + 'static
{
    pub fn new(mut rx: watch::Receiver<T>) -> WsGenInput<T> {

        let (bcast_tx, bcast_rx) = broadcast::channel(10);
        let current_value = rx.borrow_and_update().clone();
        bcast_tx.send(current_value.clone()).expect("failed to broadcast initial value");

        tokio::spawn(async move {
            loop {
                let update = rx.borrow_and_update().clone();
                bcast_tx.send(update).expect("failed to broadcast updated value");
                if rx.changed().await.is_err() {
                    break;
                }
            }
        });

        WsGenInput{
            current_value: current_value.clone(),
            rx: bcast_rx
        }
    }
}


impl Input<bool> for WsGenInput<bool> {
    fn source(&self) -> InputSource<bool> { 
        InputSource{
            start: self.current_value,
            rx: self.rx.resubscribe()
        }
     }
}

impl Input<f64> for WsGenInput<f64> {
    fn source(&self) -> InputSource<f64> { 
        InputSource{
            start: self.current_value,
            rx: self.rx.resubscribe()
        }
     }
}

impl Input<String> for WsGenInput<String> {
    fn source(&self) -> InputSource<String> { 
        InputSource{
            start: self.current_value.clone(),
            rx: self.rx.resubscribe()
        }
     }
}