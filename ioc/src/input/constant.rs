use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::warn;

use crate::{Input,InputSource};

pub struct ConstantInput {
    v: f64,
    tx: broadcast::Sender<f64>,
    rx: broadcast::Receiver<f64>
}

impl ConstantInput {
    pub fn new(value: f64) -> ConstantInput {

        let (tx, rx) = broadcast::channel(128);

        match tx.send(value) {
            Ok(_) => {
                ConstantInput{
                    v: value,
                    tx: tx,
                    rx: rx
                }
            },
            Err(e) => {
                warn!("err! {:?}", e);
                panic!();
            }
        }
    }

    pub fn set(&mut self, x: f64) {
        self.v = x;

        match self.tx.send(x) {
            Ok(_) => {},
            Err(e) => {
                warn!("err! {:?}", e);
                panic!();
            }
        }
    }
}

impl Input<f64> for ConstantInput {
    fn source(&self) -> InputSource<f64> {
        InputSource{
            start: self.v,
            rx: self.rx.resubscribe()
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ConstantInputConfig {
    value: f64
}

impl Into<Box<dyn Input<f64>>> for &ConstantInputConfig {
    fn into(self) -> Box<dyn Input<f64>> {
        let input = ConstantInput::new(self.value);
        Box::new(input)
    }
}