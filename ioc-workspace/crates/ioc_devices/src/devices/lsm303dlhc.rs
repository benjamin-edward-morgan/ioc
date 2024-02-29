
use std::{sync::{Arc, Mutex}, time::Duration};

use ioc_core::{Input,InputSource};
use embedded_hal_0::blocking::i2c;

use lsm303dlhc::Lsm303dlhc;
use tokio::{sync::broadcast, time::sleep};
use tracing::{info, warn};

pub struct VectorInput {
    value: Arc<Mutex<(f64,f64,f64)>>,
    rx: broadcast::Receiver<(f64, f64, f64)>,
}

impl VectorInput {
    pub fn new(rx: broadcast::Receiver<(f64,f64,f64)>) -> Self {
        VectorInput { value: Arc::new(Mutex::new((0.0, 0.0, 0.0))), rx: rx }
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

pub struct Lsm303dlhcDevice {
    pub accelerometer: VectorInput,
    pub magnetometer: VectorInput,
}

impl Lsm303dlhcDevice {
    pub fn new<I2C, E>(i2c: I2C) -> Result<Self,E> 
    where
        E: std::fmt::Debug,
        I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    {

        let mut device = Lsm303dlhc::new(i2c)?;

        device.mag_odr(lsm303dlhc::MagOdr::Hz30)?;
        device.accel_odr(lsm303dlhc::AccelOdr::Hz25)?;

        let (accel_tx, accel_rx) = broadcast::channel(10);
        let (mag_tx, mag_rx) = broadcast::channel(10);
        let accel_scale = 2.0 / (1 << 15) as f64;  //to scale outputs to gs 
        let mag_scale = 1.3 / (1 << 15) as f64; //to scale to milligaus
        tokio::spawn(async move {
            loop {
                match device.accel() {
                    Ok(accel) => {
                        let vector = (accel.x as f64 * accel_scale, accel.y as f64 * accel_scale, accel.z as f64 * accel_scale);
                        accel_tx.send(vector).unwrap();
                    },
                    Err(err) => {
                        warn!("device error! {:?}", err)
                    }
                }
                match device.mag() {
                    Ok(mag) => {
                        let vector = (mag.x as f64 * mag_scale, mag.y as f64 * mag_scale, mag.z as f64 * mag_scale);
                        mag_tx.send(vector).unwrap();
                    },
                    Err(err) => {
                        warn!("device error! {:?}", err)
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        });
        let accelerometer = VectorInput::new(accel_rx);       
        let magnetometer = VectorInput::new(mag_rx);

        Ok(Self {
            accelerometer,
            magnetometer,
        })
        
    }
}