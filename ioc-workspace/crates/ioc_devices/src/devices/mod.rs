use std::sync::{Arc, Mutex};
use ioc_core::{Input, InputSource};
use tokio::sync::broadcast;

#[cfg(feature = "pca9685")]
pub mod pca9685;

#[cfg(feature = "lsm303agr")]
pub mod lsm303agr;

#[cfg(feature = "lsm303dlhc")]
pub mod lsm303dlhc;

#[cfg(feature = "l3gd20")]
pub mod l3gd20;

#[cfg(feature = "bmp180")]
pub mod bmp180;

pub struct VectorInput {
    value: Arc<Mutex<(f64,f64,f64)>>,
    rx: broadcast::Receiver<(f64, f64, f64)>,
}

impl VectorInput {
    //TODO: task that updates value 
    pub fn new(rx: broadcast::Receiver<(f64,f64,f64)>) -> Self {
        VectorInput { value: Arc::new(Mutex::new((f64::NAN, f64::NAN, f64::NAN))), rx: rx }
    }
}

impl Input<(f64, f64, f64)> for VectorInput {
    fn source(&self) -> InputSource<(f64, f64, f64)> {
        let current_val = match self.value.lock() {
            Ok(current_val) => {
                current_val.to_owned()
            },
            Err(mut poisoned) => {
                poisoned.get_mut().clone()
            }
        };

        InputSource { start: current_val, rx: self.rx.resubscribe() }
    }
}

pub struct ScalerInput {
    value: Arc<Mutex<f64>>,
    rx: broadcast::Receiver<f64>,
}

impl ScalerInput {
    //TODO: task that updates value 
    pub fn new(rx: broadcast::Receiver<f64>) -> Self {
        Self{ value: Arc::new(Mutex::new(f64::NAN)), rx: rx }
    }
}

impl Input<f64> for ScalerInput {
    fn source(&self) -> InputSource<f64> {
        let current_val = match self.value.lock() {
            Ok(current_val) => {
                current_val.to_owned()
            },
            Err(mut poisoned) => {
                poisoned.get_mut().clone()
            }
        };

        InputSource { start: current_val, rx: self.rx.resubscribe() }
    }
}