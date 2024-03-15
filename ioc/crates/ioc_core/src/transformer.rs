//!Basic included transformers

use crate::{error::IocBuildError, Input, InputKind, InputSource, Transformer, TransformerI};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};

///An `Input` that is the sum of all provided `Input`s. Provided inputs must be Floats
pub struct SumInput {
    values: Arc<Mutex<Vec<f64>>>,
    rx: broadcast::Receiver<f64>,
}
struct IndexedUpdate {
    idx: usize,
    value: f64,
}

impl SumInput {
    ///create a new SumInput from a slice of `Input`s
    pub fn new(channel_size: usize, inputs: &[&dyn Input<f64>]) -> Self {
        let mut start_values = Vec::with_capacity(inputs.len());
        let mut receivers = Vec::with_capacity(inputs.len());

        for input in inputs {
            let src = input.source();
            start_values.push(src.start);
            receivers.push(src.rx);
        }

        let values_mtx = Arc::new(Mutex::new(start_values));
        let (tx, rx) = broadcast::channel(channel_size);

        let (idx_tx, mut idx_rx) = mpsc::channel(channel_size);

        //spawn a task for each input, send IndexedUpdates to idx_rx
        for (idx, mut receiver) in receivers.into_iter().enumerate() {
            let idx_tx = idx_tx.clone();
            tokio::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(value) => {
                            idx_tx
                                .send(IndexedUpdate { idx, value })
                                .await
                                .expect("failed to send!");
                        }
                        Err(broadcast::error::RecvError::Lagged(i)) => {
                            warn!("input lagged! {}", i);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                info!("sum input {} is shutting down!", idx);
            });
        }

        //spawn a task that receives messages from tdx_tx,
        //updates the indexed values,
        //recomputes sum,
        //and emits the new sum
        let values = values_mtx.clone();
        tokio::spawn(async move {
            while let Some(update) = idx_rx.recv().await {
                match values.lock() {
                    Ok(mut values) => {
                        values[update.idx] = update.value;
                        let sum = values.iter().sum();
                        if let Err(err) = tx.send(sum) {
                            error!("error sending! {:?}", err);
                            break;
                        }
                    }
                    Err(mut poisoned) => {
                        let values = poisoned.get_mut();
                        values[update.idx] = update.value;
                        let sum = values.iter().sum();
                        if let Err(err) = tx.send(sum) {
                            error!("error sending! {:?}", err);
                            break;
                        }
                    }
                }
            }
            info!("sum input fan-in task is shutting down!");
        });

        Self {
            values: values_mtx,
            rx,
        }
    }
}

impl Input<f64> for SumInput {
    fn source(&self) -> InputSource<f64> {
        match self.values.lock() {
            Ok(values) => InputSource {
                start: values.iter().sum(),
                rx: self.rx.resubscribe(),
            },
            Err(poisoned) => {
                let values = poisoned.get_ref();
                InputSource {
                    start: values.iter().sum(),
                    rx: self.rx.resubscribe(),
                }
            }
        }
    }
}

pub struct SumConfig<'a> {
    pub inputs: Vec<&'a dyn Input<f64>>,
}

///Sum Transformer
pub struct Sum {
    pub value: SumInput,
}

impl From<Sum> for TransformerI {
    fn from(val: Sum) -> Self {
        TransformerI {
            join_handle: tokio::spawn(async move { println!("this is wrong") }), //todo
            inputs: HashMap::from([("value".to_string(), InputKind::float(val.value))]),
        }
    }
}

impl<'a> Transformer<'a> for Sum {
    type Config = SumConfig<'a>;

    async fn try_build(cfg: &SumConfig<'a>) -> Result<Sum, IocBuildError> {
        Ok(Sum {
            value: SumInput::new(10, &cfg.inputs),
        })
    }
}
