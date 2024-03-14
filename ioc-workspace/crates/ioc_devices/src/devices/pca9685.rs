
use std::{collections::HashMap, sync::{Arc, Mutex}};

use futures::future::join_all;
use ioc_core::{error::IocBuildError, ModuleBuilder, ModuleIO, Output, OutputKind, OutputSink};
use pwm_pca9685::{Address, Pca9685, Channel};
use embedded_hal_0::blocking::i2c;
use serde::Deserialize;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::error::DeviceConfigError;

use tracing::{error,info};

//system level config -- corresponds to 1 pwm chip instance 
#[derive(Debug, Deserialize)]
pub struct Pca9685DeviceConfig {
    pub i2c_address: u8,
    pub channels: HashMap<String, u8>
}

//connected pwm chip instance
pub struct Pca9685Device {
    pub join_handle: JoinHandle<()>,
    pub channels: HashMap<String, Pca9685PwmOutput>
}

impl From<Pca9685Device> for ModuleIO {
    fn from(dev: Pca9685Device) -> Self {
        ModuleIO { 
            join_handle: dev.join_handle, 
            inputs: HashMap::new(), 
            outputs: dev.channels.into_iter()
                .map(|(key, out)| (key, OutputKind::float(out)))
                .collect()
        }
    }
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

impl Pca9685Device {
    pub fn build<I2C, E>(config: &Pca9685DeviceConfig, i2c: I2C) -> Result<Pca9685Device,DeviceConfigError>  
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
        let mut join_handles: Vec<JoinHandle<()>> = Vec::with_capacity(config.channels.len());
        for (k, c) in &config.channels {
            let (output, join_handle) = Pca9685PwmOutput::try_build(device.clone(), *c)?;
            join_handles.push(join_handle);
            channels.insert(k.to_string(), output);
        }

        let join_handle = tokio::spawn(async move {
            join_all(join_handles).await;
            info!("pca 9685 is done!")
        });

        Ok(Pca9685Device { 
            join_handle,
            channels
        })
    }
}




pub struct Pca9685DeviceBuilder<E, I2C, F>
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    i2c_bus_provider: F,
}

impl <E, I2C, F> Pca9685DeviceBuilder<E, I2C, F> 
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    pub fn new(i2c_bus_provider: F) -> Pca9685DeviceBuilder<E, I2C, F> {
        Pca9685DeviceBuilder { 
            i2c_bus_provider,
        }
    }
}


impl <E, I2C, F> ModuleBuilder for Pca9685DeviceBuilder<E, I2C, F> 
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    type Config = Pca9685DeviceConfig;
    type Module = Pca9685Device; 

    async fn try_build(&self, cfg: &Pca9685DeviceConfig) -> Result<Pca9685Device, IocBuildError> {
        let i2c = (self.i2c_bus_provider)(1);
        let dev = Pca9685Device::build(cfg, i2c)?;
        Ok(dev)
    }

}


//pwm float output associated with a channel (pin) on a Pca9685Device
//clamped to [0.0, 1.0] which maps to a duty cycle from 0 to 100%
pub struct Pca9685PwmOutput {
    tx: mpsc::Sender<f64>
}

impl Pca9685PwmOutput {
    fn try_build<I2C, E>(device: Arc<Mutex<Pca9685<I2C>>>, channel: u8) -> Result<(Self, JoinHandle<()>), DeviceConfigError>
        where
            E: std::fmt::Debug,
            I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<f64>(100);
        let chann = Channel::try_from(channel)
            .map_err(|_| DeviceConfigError::new(format!("Invalid channel for PCA9685 {}", channel)))?;
        
        let device = device.clone();
        let join_handle = tokio::spawn(async move {
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

        Ok((Self { tx, }, join_handle))
    }
}

impl Output<f64> for Pca9685PwmOutput {
    fn sink(&self) -> OutputSink<f64> {
        OutputSink { 
            tx: self.tx.clone() 
        }
    }
}
