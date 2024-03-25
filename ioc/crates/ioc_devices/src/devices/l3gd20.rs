use std::{collections::HashMap, time::Duration};

use embedded_hal::i2c;
use ioc_core::{error::IocBuildError, Input, InputKind, ModuleBuilder, ModuleIO, Value};
use serde::Deserialize;
use tokio::{sync::watch, task::JoinHandle, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub enum DataRate {
    Low0,
    Low1,
    Low2,
    High0,
    High1,
    High2,
    High3,
}

impl DataRate {
    fn odr_hertz(&self) -> f64 {
        match *self {
            Self::Low0 => 12.5,
            Self::Low1 => 25.0,
            Self::Low2 => 50.0,
            Self::High0 => 100.0,
            Self::High1 => 200.0,
            Self::High2 => 400.0,
            Self::High3 => 800.0,
        }
    }

    fn dr_bits(&self) -> u8 {
        match *self {
            Self::Low0 | Self::High0 => 0,
            Self::Low1 | Self::High1 => 1,
            Self::Low2 | Self::High2 => 2,
            Self::High3 => 3,
        }
    }

    fn _low_odr_bit(&self) -> u8 {
        match *self {
            Self::Low0 | Self::Low1 | Self::Low2 => 1,
            Self::High0 | Self::High1 | Self::High2 | Self::High3 => 0,
        }
    }
}

pub enum DataScale {
    _245DegPerSec,
    _500DegPerSec,
    _2000DegPerSec,
}

impl DataScale {
    fn scale_output_dps(&self, data: i16) -> f64 {
        match *self {
            Self::_245DegPerSec => (data as f64) / ((2 << 15) as f64) * 245.0,
            Self::_500DegPerSec => (data as f64) / ((2 << 15) as f64) * 500.0,
            Self::_2000DegPerSec => (data as f64) / ((2 << 15) as f64) * 2000.0,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct L3gd20DeviceConfig {
    pub i2c_address: u8,
}

impl Default for L3gd20DeviceConfig {
    fn default() -> Self {
        Self { i2c_address: 0x6B }
    }
}

pub struct L3gd20Device {
    pub join_handle: JoinHandle<()>,
    pub gyroscope: Input<Vec<Value>>,
}

impl L3gd20Device {
    pub fn try_build<I2C>(config: &L3gd20DeviceConfig, mut i2c: I2C, cancel_token: CancellationToken) -> Result<Self, IocBuildError>
    where
        I2C: i2c::I2c + Send + 'static,
    {
        //check that the i2c address is valid
        let mut has_valid_address = false;
        let mut i = 0;
        while i < VALID_I2C_ADDRESSES.len() && !has_valid_address {
            has_valid_address = VALID_I2C_ADDRESSES[i] == config.i2c_address;
            i += 1;
        }
        if !has_valid_address {
            return Err(IocBuildError::from_string(format!(
                "Invalid I2C address for L3gd20."
            )));
        }

        //read the id register from the device and make sure it's what we expect
        let mut buffer = [0u8; 1];
        if let Err(err) = i2c.write_read(config.i2c_address, &[ID_REGISTER], &mut buffer) {
            return Err(IocBuildError::from_string(format!(
                "Error reading id from L3gd20. {:?}",
                err
            )));
        }
        if buffer[0] != EXPECTED_ID {
            return Err(
                IocBuildError::from_string(format!("Got invalid id from L3gd20 device register. Got {} but expected {}. Another device may be connected.", buffer[0], EXPECTED_ID))
            );
        }

        let (gyro, tx) = Input::new(Vec::new());

        let join_handle = spawn_gyro_task(config.i2c_address, tx, i2c, cancel_token);

        Ok(Self {
            join_handle: join_handle,
            gyroscope: gyro,
        })
    }
}

impl From<L3gd20Device> for ModuleIO {
    fn from(dev: L3gd20Device) -> ModuleIO {
        ModuleIO {
            join_handle: dev.join_handle,
            inputs: HashMap::from([("value".to_string(), InputKind::Array(dev.gyroscope))]),
            outputs: HashMap::new(),
        }
    }
}

pub struct L3gd20DeviceBuilder<I2C, F>
where
    I2C: i2c::I2c + Send + 'static,
    F: Fn(u8) -> I2C,
{
    i2c_bus_provider: F,
}

impl<I2C, F> L3gd20DeviceBuilder<I2C, F>
where
    I2C: i2c::I2c + Send + 'static,
    F: Fn(u8) -> I2C,
{
    pub fn new(i2c_bus_provider: F) -> Self {
        L3gd20DeviceBuilder { i2c_bus_provider }
    }
}

impl<I2C, F> ModuleBuilder for L3gd20DeviceBuilder<I2C, F>
where
    I2C: i2c::I2c + Send + 'static,
    F: Fn(u8) -> I2C,
{
    type Config = L3gd20DeviceConfig;
    type Module = L3gd20Device;

    async fn try_build(&self, cfg: &L3gd20DeviceConfig, cancel_token: CancellationToken) -> Result<L3gd20Device, IocBuildError> {
        L3gd20Device::try_build(cfg, (self.i2c_bus_provider)(1), cancel_token)
    }
}

const VALID_I2C_ADDRESSES: [u8; 2] = [0x6B, 0x6A];
const EXPECTED_ID: u8 = 0xD7;
const MULTI_READ_MASK: u8 = 0x80;

const ID_REGISTER: u8 = 0x0F;
const CTRL1_REGISTER: u8 = 0x20;
const _OUT_X_LSB_REGISTER: u8 = 0x28;
const OUT_TEMP_REGISTER: u8 = 0x26;

fn ctrl1_register_value(dr: &DataRate, bw: u8, enabled: bool) -> u8 {
    (dr.dr_bits() << 6) | ((bw & 0b11) << 4) | (if enabled { 0b1111 } else { 0b0 })
}

fn spawn_gyro_task<I2C>(
    i2c_address: u8,
    tx: watch::Sender<Vec<Value>>,
    mut i2c: I2C,
    cancel_token: CancellationToken
) -> JoinHandle<()>
where
    I2C: i2c::I2c + Send + 'static,
{
    let dr = DataRate::High0;
    let bw: u8 = 0;
    let scale = DataScale::_245DegPerSec;

    tokio::spawn(async move {
        let ctrl1 = ctrl1_register_value(&dr, bw, true);

        if let Err(err) = i2c.write(i2c_address, &[CTRL1_REGISTER, ctrl1]) {
            warn!("error setting control! {:?}", err);
            panic!("gyro not working");
        }

        let mut buffer = [0u8; 5];
        i2c.write_read(
            i2c_address,
            &[CTRL1_REGISTER | MULTI_READ_MASK],
            &mut buffer,
        )
        .unwrap();

        loop {
            if cancel_token.is_cancelled() {
                break;
            }
            let mut buffer = [0u8; 8];
            if let Err(err) = i2c.write_read(
                i2c_address,
                &[OUT_TEMP_REGISTER | MULTI_READ_MASK],
                &mut buffer,
            ) {
                error!("error reading gyroscope data! {:?}", err);
                break;
            }

            let x = scale.scale_output_dps(((buffer[3] as i16) << 8) | (buffer[2] as i16));
            let y = scale.scale_output_dps(((buffer[5] as i16) << 8) | (buffer[4] as i16));
            let z = scale.scale_output_dps(((buffer[7] as i16) << 8) | (buffer[6] as i16));

            if let Err(err) = tx.send(vec![Value::Float(x), Value::Float(y), Value::Float(z)]) {
                warn!("error sending gyroscope data! {:?}", err);
                break;
            }

            sleep(Duration::from_millis((1000.0 / dr.odr_hertz()) as u64)).await;
        }

        debug!("shutting down gyro!");

        let ctrl1 = ctrl1_register_value(&dr, 0, false);
        match i2c.write(i2c_address, &[CTRL1_REGISTER, ctrl1]) {
            Ok(_) => debug!("successfullt disabled gyro on shutdown"),
            Err(err) => warn!("error disabling gyro on shutdown! {:?}", err),
        }

    })
}
