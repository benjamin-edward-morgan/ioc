use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio::task::JoinHandle;
use tracing::warn;

use crate::{Input,InputSource};

pub struct NoiseInput {
    pub handle: JoinHandle<()>,
    rx: broadcast::Receiver<f64>
}

impl NoiseInput {
    pub fn new(min: f64, max: f64, period_ms: u64) -> NoiseInput {
        let (tx, rx) = broadcast::channel(128);

        let m = max - min;
        let b = min;
        let handle = tokio::spawn( async move {

            while let Ok(_subscribers) = tx.send(rand::random::<f64>()*m + b) {
                sleep(Duration::from_millis(period_ms)).await;
            }
            warn!("noise input shutting down");
        });

        NoiseInput {
            handle: handle,
            rx: rx
        }
    }
}

impl Input<f64> for NoiseInput {
    fn source(&self) -> InputSource<f64> { 
        InputSource{
            start: 0.0,
            rx: self.rx.resubscribe()
        }
     }
}

#[derive(Deserialize, Debug)]
pub struct NoiseInputConfig {
    min: f64,
    max: f64,
    period_ms: u64
}

impl Into<Box<dyn Input<f64>>> for &NoiseInputConfig {
    fn into(self) -> Box<dyn Input<f64>> {
        let input = NoiseInput::new(self.min, self.max, self.period_ms);
        Box::new(input)
    }
}