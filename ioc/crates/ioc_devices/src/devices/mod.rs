use ioc_core::{Input, InputSource, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

///The PCA9685 is an i2c device from NXP with 16 pwm output channels with 12 bit resolution.
#[cfg(feature = "pca9685")]
pub mod pca9685;

///The LSM303DLHC is an i2c device from STMicroelectronics. It includes a MEMS accelerometer and a MEMS magnetometer.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "lsm303dlhc")]
pub mod lsm303dlhc;

///The L3GD20 is an i2c device from STMicroelectronics with a MEMS gyroscope sensor.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "l3gd20")]
pub mod l3gd20;

///The BMP180 is an i2c device from Bosch Sensortec. It includes an ambient temperature sensor and an atmospheric pressure senso.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "bmp180")]
pub mod bmp180;

pub struct VectorInput {
    value: Arc<Mutex<Vec<Value>>>,
    rx: broadcast::Receiver<Vec<Value>>,
}

impl VectorInput {
    //TODO: task that updates value
    pub fn new(rx: broadcast::Receiver<Vec<Value>>) -> Self {
        VectorInput {
            value: Arc::new(Mutex::new(Vec::new())),
            rx,
        }
    }
}

impl Input<Vec<Value>> for VectorInput {
    fn source(&self) -> InputSource<Vec<Value>> {
        let current_val = match self.value.lock() {
            Ok(current_val) => current_val.to_owned(),
            Err(poisoned) => poisoned.get_ref().to_vec(),
        };

        InputSource {
            start: current_val,
            rx: self.rx.resubscribe(),
        }
    }
}

pub struct ScalerInput {
    value: Arc<Mutex<f64>>,
    rx: broadcast::Receiver<f64>,
}

impl ScalerInput {
    //TODO: task that updates value
    pub fn new(rx: broadcast::Receiver<f64>) -> Self {
        Self {
            value: Arc::new(Mutex::new(f64::NAN)),
            rx,
        }
    }
}

impl Input<f64> for ScalerInput {
    fn source(&self) -> InputSource<f64> {
        let current_val = match self.value.lock() {
            Ok(current_val) => current_val.to_owned(),
            Err(mut poisoned) => **poisoned.get_mut(),
        };

        InputSource {
            start: current_val,
            rx: self.rx.resubscribe(),
        }
    }
}
