use std::{collections::HashMap, time::Duration};

use embedded_hal_0::blocking::i2c;
use ioc_core::{error::IocBuildError, Input, InputKind, ModuleBuilder, ModuleIO, Value};

use lsm303dlhc::Lsm303dlhc;
use serde::Deserialize;
use tokio::{task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

#[derive(Deserialize, Debug)]
pub struct Lsm303dlhcDeviceConfig {}

pub struct Lsm303dlhcDevice {
    pub join_handle: JoinHandle<()>,
    pub accelerometer: Input<Vec<Value>>,
    pub magnetometer: Input<Vec<Value>>,
}

impl Lsm303dlhcDevice {
    pub fn try_build<I2C, E>(_cfg: &Lsm303dlhcDeviceConfig, i2c: I2C, cancel_token: CancellationToken) -> Result<Self, E>
    where
        E: std::fmt::Debug,
        I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    {
        let mut device = Lsm303dlhc::new(i2c)?;

        device.mag_odr(lsm303dlhc::MagOdr::Hz30)?;
        device.accel_odr(lsm303dlhc::AccelOdr::Hz25)?;

        let accel_scale = 2.0 / (1 << 15) as f64; //to scale outputs to gs
        let mag_scale = 1.3 / (1 << 15) as f64; //to scale to milligaus

        let (accelerometer, accel_tx) = Input::new(Vec::new());
        let (magnetometer, mag_tx) = Input::new(Vec::new());

        let join_handle = tokio::spawn(async move {
            loop {
                if cancel_token.is_cancelled() {
                    break;
                }
                match device.accel() {
                    Ok(accel) => {
                        let vector = (
                            accel.x as f64 * accel_scale,
                            accel.y as f64 * accel_scale,
                            accel.z as f64 * accel_scale,
                        );
                        let vector = vec![
                            Value::Float(vector.0),
                            Value::Float(vector.1),
                            Value::Float(vector.2),
                        ];
                        accel_tx.send(vector).unwrap();
                    }
                    Err(err) => {
                        warn!("device error! {:?}", err)
                    }
                }
                match device.mag() {
                    Ok(mag) => {
                        let vector = (
                            mag.x as f64 * mag_scale,
                            mag.y as f64 * mag_scale,
                            mag.z as f64 * mag_scale,
                        );
                        let vector = vec![
                            Value::Float(vector.0),
                            Value::Float(vector.1),
                            Value::Float(vector.2),
                        ];
                        mag_tx.send(vector).unwrap();
                    }
                    Err(err) => {
                        warn!("device error! {:?}", err)
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }

            debug!("shutting down lsm303dlhc!");
        });

        Ok(Self {
            join_handle,
            accelerometer,
            magnetometer,
        })
    }
}

impl From<Lsm303dlhcDevice> for ModuleIO {
    fn from(dev: Lsm303dlhcDevice) -> Self {
        ModuleIO {
            join_handle: dev.join_handle,
            inputs: HashMap::from([
                (
                    "accelerometer".to_string(),
                    InputKind::Array(dev.accelerometer),
                ),
                (
                    "magnetometer".to_string(),
                    InputKind::Array(dev.magnetometer),
                ),
            ]),
            outputs: HashMap::new(),
        }
    }
}

pub struct Lsm303dlhcDeviceBuilder<E, I2C, F>
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    i2c_bus_provider: F,
}

impl<E, I2C, F> Lsm303dlhcDeviceBuilder<E, I2C, F>
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    pub fn new(i2c_bus_provider: F) -> Self {
        Lsm303dlhcDeviceBuilder { i2c_bus_provider }
    }
}

impl<E, I2C, F> ModuleBuilder for Lsm303dlhcDeviceBuilder<E, I2C, F>
where
    E: std::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::WriteRead<Error = E> + Send + 'static,
    F: Fn(u8) -> I2C,
{
    type Config = Lsm303dlhcDeviceConfig;
    type Module = Lsm303dlhcDevice;

    async fn try_build(&self, cfg: &Self::Config, cancel_token: CancellationToken) -> Result<Self::Module, IocBuildError> {
        Lsm303dlhcDevice::try_build(cfg, (self.i2c_bus_provider)(1), cancel_token).map_err(|err| {
            IocBuildError::from_string(format!("Error building Lsm303dlhc device: {:?}", err))
        })
    }
}
