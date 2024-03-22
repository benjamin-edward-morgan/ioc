//!Basic included transformers

use crate::{error::IocBuildError, Input, InputKind, Transformer, TransformerI};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{sync::{broadcast, mpsc}, task::JoinHandle};
use tracing::{error, info, warn};


pub struct SumConfig {
    pub inputs: Vec<Input<f64>>,
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

impl Transformer<'_> for Sum {
    type Config = SumConfig;

    async fn try_build(cfg: &SumConfig) -> Result<Sum, IocBuildError> {
        let (value, join_handle) = spawn_sum_task(&cfg.inputs);
        Ok(Sum {value, join_handle})
    }
}

fn spawn_sum_task(inputs: &[Input<f64>]) -> (Input<f64>, JoinHandle<()>) {
    
    let mut values = Vec::with_capacity(inputs.len());
    let mut sources = Vec::with_capacity(inputs.len());

    for (idx, input) in inputs.into_iter().enumerate() {
        let mut source = input.source();
        let value: f64 = *source.borrow_and_update();
        sources.push(source);
        values.push(value);

    }

    let (sum, sum_tx) = Input::new(values.iter().sum::<f64>());

    sum_tx

    let join_handles = Vec::with_capacity(inputs.len());


    todo!()
}