

use ioc_core::{Input, InputSource};
use embedded_hal::i2c;
use tokio::sync::broadcast;
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr};

use crate::error::DeviceConfigError;

use tracing::{error,warn,info};

pub struct Lsm303DeviceConfig {
    
}

pub struct InputVector {
    pub x: Lsm303Input,
    pub y: Lsm303Input,
    pub z: Lsm303Input,
}

pub struct Lsm303Device {
    pub accelerometer: InputVector,
}

impl Lsm303Device {
    pub fn build<I2C>(config: Lsm303DeviceConfig, i2c: I2C) -> Result<Lsm303Device, DeviceConfigError> 
    where
        I2C: i2c::I2c,
    {
        let mut device = Lsm303agr::new_with_i2c(i2c);
        device.init().unwrap();
       // device.set_accel_mode_and_odr(&mut Delay, AccelMode::Normal, AccelOutputDataRate::Hz10).unwrap();

       let x = Lsm303Input::new();
       let y = Lsm303Input::new();
       let z = Lsm303Input::new();
       
        Ok(Self{
            accelerometer: InputVector { x, y, z }
        })
    }
}

pub struct Lsm303Input {

}

impl Lsm303Input {
    fn new() -> Self {
        Self {}
    }
}

impl Input<f64> for Lsm303Input {
    fn source(&self) -> InputSource<f64> {
        todo!()
        //InputSource { start: (), rx: () }
    }
}