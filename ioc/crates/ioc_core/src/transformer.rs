//!Basic included transformers

use crate::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use std::collections::HashMap;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{debug, error, warn};


pub struct SumConfig<'a> {
    pub inputs: &'a [&'a Input<f64>],
}

///Sum Transformer
pub struct Sum {
    pub join_handle: JoinHandle<()>,
    pub value: Input<f64>,
}

impl From<Sum> for TransformerI {
    fn from(sum: Sum) -> Self {
        TransformerI {
            join_handle: sum.join_handle,
            inputs: HashMap::from([
                ("value".to_string(), InputKind::Float(sum.value))
            ]),
        }
    }
}

impl <'a> Transformer<'a> for Sum {
    type Config = SumConfig<'a>;

    async fn try_build(cfg: &SumConfig<'a>) -> Result<Sum, IocBuildError> {
        let (value, join_handle) = spawn_sum_task(cfg.inputs);
        Ok(Sum {value, join_handle})
    }
}

struct IndexedUpdate{
    idx: usize,
    new_value: f64,
}

fn spawn_sum_task(inputs: &[&Input<f64>]) -> (Input<f64>, JoinHandle<()>) {

    //get sources and current values for each input
    let mut values = Vec::with_capacity(inputs.len());
    let mut sources = Vec::with_capacity(inputs.len());
    for input in inputs {
        let mut source = input.source();
        let value: f64 = *source.borrow_and_update();
        sources.push(source);
        values.push(value);

    }
    //create an input that is the sum of values
    let (sum, sum_tx) = Input::new(values.iter().sum::<f64>());

    //mpsc channel for `IndexedUpdate`s
    let (update_tx, mut update_rx) = mpsc::channel(1);

    //task which receives `IndexedUpdate`s and emits updates to the sum input
    let sum_handle = tokio::spawn(async move {
        while let Some(IndexedUpdate{ idx, new_value }) = update_rx.recv().await {
            values[idx] = new_value;
            let new_sum = values.iter().sum();
            if let Err(err) = sum_tx.send(new_sum) {
                error!("Error sending to sum: {}", err);
                break;
            }
        }
        debug!("shutting down sum task!")
    });

    //create a task for each input to listen for changes and send `IndexedUpdate`s to the sum task
    let mut join_handles = Vec::with_capacity(inputs.len());
    join_handles.push(sum_handle);
    for (idx, mut source) in sources.into_iter().enumerate() {
        let update_tx = update_tx.clone();
        let handle = tokio::spawn(async move {
            while source.changed().await.is_ok() {
                let new_value: f64 = *source.borrow_and_update();
                let update = IndexedUpdate{ idx, new_value };
                if let Err(err) = update_tx.send(update).await {
                    warn!("Error sending to sum: {}", err);
                    break;
                }
            }
            debug!("shutting down sum source task {idx} !")
        });
        join_handles.push(handle);
    }

    //join all join_handles in a single task
    let join_handle = tokio::spawn(async move {
        for join_handle in join_handles {
            join_handle.await.unwrap();
        }
    });

    (sum, join_handle)
}