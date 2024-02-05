
use std::{collections::HashMap, sync::{Arc, Mutex}};

use ioc_core::{Output,OutputSink};
use pwm_pca9685::{Address, Pca9685, Channel};
use embedded_hal_0::blocking::i2c;
use tokio::sync::mpsc;

use crate::error::DeviceConfigError;

use tracing::{error,warn,info};

//system level config -- corresponds to 1 pwm chip instance 
#[derive(Debug)]
pub struct Pca9685DeviceConfig<'a> {
    pub i2c_address: u8,
    pub channels: HashMap<&'a str, u8>
}

//connected pwm chip instance
pub struct Pca9685Device<'a> {
    pub channels: HashMap<&'a str, Pca9685PwmOutput>
}

impl <E> From<pwm_pca9685::Error<E>> for DeviceConfigError 
where
E: std::fmt::Debug,
{
    fn from(err: pwm_pca9685::Error<E>) -> DeviceConfigError {
        let message = match err {
            pwm_pca9685::Error::I2C(err) => format!("Could not configure PCA9685 Device: {:?}", err),
            pwm_pca9685::Error::InvalidInputData => "PCA9685: Invalid input data".to_string()
        };
        DeviceConfigError::new(message)
    }
}

impl Pca9685Device<'_>
{
    pub fn build<I2C, E>(config: Pca9685DeviceConfig, i2c: I2C) -> Result<Pca9685Device,DeviceConfigError>  
    where
        E: std::fmt::Debug,
        I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    {
        let address = Address::from(config.i2c_address);
        let mut device = Pca9685::new(i2c, address)?;
            
        device.set_prescale(100)?;
        device.enable()?;

        let device = Arc::new(Mutex::new(device));

        let mut channels = HashMap::with_capacity(config.channels.len());
        for (k, c) in config.channels {
            let output = Pca9685PwmOutput::try_build(device.clone(), c)?;
            channels.insert(k, output);
        }

        Ok(Pca9685Device { 
            channels
        })
    }
}


//pwm float output associated with a channel (pin) on a Pca9685Device
//clamped to [0.0, 1.0] which maps to a duty cycle from 0 to 100%
pub struct Pca9685PwmOutput {
    tx: mpsc::Sender<f64>
}

impl Pca9685PwmOutput {
    fn try_build<I2C, E>(device: Arc<Mutex<Pca9685<I2C>>>, channel: u8) -> Result<Self, DeviceConfigError>
        where
            E: std::fmt::Debug,
            I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<f64>(100);
        let chann = Channel::try_from(channel)
            .map_err(|_| DeviceConfigError::new(format!("Invalid channel for PCA9685 {}", channel)))?;
        
        let device = device.clone();
        tokio::spawn(async move {
            while let Some(new_value) = rx.recv().await { 
                // info!("new val: {}", new_value);
                let off_time = (new_value.min(1.0).max(0.0) * 4095.0) as u16;
                let mut device = match device.lock() {
                    Ok(device) => device,
                    Err(poisoned) => poisoned.into_inner(),   
                } ;
                if let Err(err) = device.set_channel_on_off(chann, 0, off_time) {
                    error!("error setting PCA9685 device output! {:?}", err);
                }
            }
            info!("Pca9685PwmOutput for channel {} shutting down.", channel);
        });

        Ok(Self { tx, })
    }
}

impl Output<f64> for Pca9685PwmOutput {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink { 
            tx: self.tx.clone() 
        }
    }
}
