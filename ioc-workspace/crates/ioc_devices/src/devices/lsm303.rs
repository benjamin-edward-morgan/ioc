

use std::{thread, time::Duration};

use ioc_core::{Input, InputSource};
use embedded_hal::i2c;
use embedded_hal::delay::DelayNs;
use tokio::sync::broadcast;
use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, MagMode, MagOutputDataRate};

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

pub struct NoopDelay;

impl DelayNs for NoopDelay {
    fn delay_ns(&mut self, ns: u32) {
        thread::sleep(Duration::from_nanos(ns as u64))
    }
}

impl Lsm303Device {
    pub fn build<I2C>(config: Lsm303DeviceConfig, i2c: I2C) -> Result<Lsm303Device, DeviceConfigError> 
    where
        I2C: i2c::I2c + Send + 'static,
    {
        let mut device = Lsm303agr::new_with_i2c(i2c);
        device.init().unwrap();

        device.mag_enable_low_pass_filter().unwrap();

        device.set_accel_mode_and_odr(&mut NoopDelay, AccelMode::Normal, AccelOutputDataRate::Hz10).unwrap();
        device.set_mag_mode_and_odr(&mut NoopDelay, MagMode::HighResolution, MagOutputDataRate::Hz10).unwrap();

        let accel_id = device.accelerometer_id().unwrap();
        println!("accelerometer id: {:?}, valid: {}", accel_id, accel_id.is_correct());

        let mag_id = device.magnetometer_id().unwrap();
        println!("magnetometer id: {:?}, valid: {}", mag_id, mag_id.is_correct());

        // tokio::spawn(async move {
        //     loop {
        //         if device.accel_status().unwrap().xyz_new_data() {
        //             let data = device.acceleration().unwrap();
        //             println!("Acceleration: x {} y {} z {}", data.x_mg(), data.y_mg(), data.z_mg());

        //             // let data = device.magnetic_field().unwrap();
        //             // println!("Mag: x {} y {} z {}", data.x_nt(), data.y_nt(), data.z_nt());
        //         }

        //         // println!("mag stat: {:?}", device.mag_status().unwrap().);

        //         // if device.mag_status().unwrap().xyz_new_data() {
                    
        //         // }

        //         // println!("temp: device.temperature().unwrap().degrees_celsius()
        //     }
        // });

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