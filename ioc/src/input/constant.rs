use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::warn;

use crate::{Input, InputSource};

pub struct ConstantInput {
    v: f64,
    _tx: broadcast::Sender<f64>,
    rx: broadcast::Receiver<f64>,
}

impl ConstantInput {
    pub fn new(value: f64) -> ConstantInput {
        let (tx, rx) = broadcast::channel(128);

        if let Err(err) = tx.send(value) {
            warn!("err! {:?}", err);
        }

        ConstantInput {
            v: value,
            _tx: tx,
            rx,
        }
    }
}

impl Input<f64> for ConstantInput {
    fn source(&self) -> InputSource<f64> {
        InputSource {
            start: self.v,
            rx: self.rx.resubscribe(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ConstantInputConfig {
    value: f64,
}

impl From<&ConstantInputConfig> for Box<dyn Input<f64>> {
    fn from(val: &ConstantInputConfig) -> Self {
        let input = ConstantInput::new(val.value);
        Box::new(input)
    }
}
