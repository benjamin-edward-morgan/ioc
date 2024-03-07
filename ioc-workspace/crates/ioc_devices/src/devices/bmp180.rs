use std::time::Duration;

use embedded_hal::i2c;
use tokio::{sync::broadcast, time::sleep};
use tracing::{error, info, warn};
use super::ScalerInput;

use crate::error::DeviceConfigError;

#[derive(Clone, Copy)]
pub enum PressurePrecision {
    UltraLowPower,
    Standard,
    HighResolution,
    UltraHighResolution,
}

pub struct Bmp180DeviceConfig {
    pressure_precision: PressurePrecision,
    period_ms: u64,
}

impl Default for Bmp180DeviceConfig {
    fn default() -> Self {
        Self { 
            pressure_precision: PressurePrecision::Standard, 
            period_ms: 1000,
        }
    }
}

pub struct Bmp180Device {
    pub temperature_c: ScalerInput,
    pub pressure_h_pa: ScalerInput,
}

const I2C_ADDRESS: u8 = 0x77;

const ID_REGISTER: u8 = 0xD0;
const AC1_MSB_REGISTER: u8 = 0xAA;
const CONTROL_REGISTER: u8 = 0xF4;
const OUT_MSB_REGISTER: u8 = 0xF6;

const MEASURE_TEMPERATURE: u8 = 0x2E;
const MEASURE_PRESS_OSS_0: u8 = 0x34;
const MEASURE_PRESS_OSS_1: u8 = 0x74;
const MEASURE_PRESS_OSS_2: u8 = 0xB4;
const MEASURE_PRESS_OSS_3: u8 = 0xF4;

const ID_EXPECTED_VALUE: u8 = 0x55;

const TEMP_WAIT: Duration = Duration::from_micros(4500);
const PRESS_0_WAIT: Duration = Duration::from_micros(4500);
const PRESS_1_WAIT: Duration = Duration::from_micros(7500);
const PRESS_2_WAIT: Duration = Duration::from_micros(13500);
const PRESS_3_WAIT: Duration = Duration::from_micros(25500);


#[derive(Debug)]
struct Bmp180CalibrationData {
    ac1: i16,
    ac2: i16, 
    ac3: i16,
    ac4: u16,
    ac5: u16,
    ac6: u16,
    b1: i16,
    b2: i16,
    mb: i16,
    mc: i16,
    md: i16,
}

struct TempResult {
    temp_c: f64,
    b5: i64,
}

impl Bmp180CalibrationData {
    fn from_bytes(bytes: &[u8]) -> Result<Self, DeviceConfigError>{
        if bytes.len() != 22 {
            Err(DeviceConfigError::from_str("Bmp180 Calibration expected 22 bytes of calibration data."))
        } else {
            let calib_data = Bmp180CalibrationData{
                ac1: ((bytes[0] as i16) << 8) | (bytes[1] as i16),
                ac2: ((bytes[2] as i16) << 8) | (bytes[3] as i16),
                ac3: ((bytes[4] as i16) << 8) | (bytes[5] as i16),
                ac4: ((bytes[6] as u16) << 8) | (bytes[7] as u16),
                ac5: ((bytes[8] as u16) << 8) | (bytes[9] as u16),
                ac6: ((bytes[10] as u16) << 8) | (bytes[11] as u16),
                b1: ((bytes[12] as i16) << 8) | (bytes[13] as i16),
                b2: ((bytes[14] as i16) << 8) | (bytes[15] as i16),
                mb: ((bytes[16] as i16) << 8) | (bytes[17] as i16),
                mc: ((bytes[18] as i16) << 8) | (bytes[19] as i16),
                md: ((bytes[20] as i16) << 8) | (bytes[21] as i16),   
            };

            Ok(calib_data)
        }
    }

    fn calc_temperature_c(&self, ut: i32) -> TempResult {
        let ut = ut as i64;
        let x1 = (ut - (self.ac6 as i64)) * (self.ac5 as i64) / (1 << 15);
        let x2 = (self.mc as i64) * (1 << 11) / (x1 + (self.md as i64));
        let b5 = x1 + x2;
        let t_tenths = (b5 + 8) / (1 << 4);
        TempResult{
            temp_c: (t_tenths as f64) / 10.0,
            b5: b5
        }
    }

    fn calc_pressure_h_pa(&self, b5: i64, oss: u8, up: i32) -> f64 {
        let up = up as i64;
        let b6 = b5 - 4000;
        let x1 = ((self.b2 as i64) * b6*b6 / (1 << 12)) / (1 << 11);
        let x2 = (self.ac2 as i64) * b6 / (1 << 11);
        let x3 = x1 + x2;
        let b3 = ((((self.ac1 as i64) * 4 + x3) << oss) + 2)/4;
        let x1 = (self.ac3 as i64) * b6 / (1 << 13);
        let x2 = ((self.b1 as i64) * b6 * b6 / (1 << 12)) / (1 << 16);
        let x3 = ((x1 + x2) + 2) / (1 << 2);
        let b4 = (self.ac4 as u64) * ((x3 + 32768) as u64) / (1 << 15);
        let b7 = ((up - b3) as u64)*(50000 >> oss);
        let p: i64;
        if b7 < 0x80000000 {
            p = (b7 * 2 / b4) as i64;
        } else {
            p = (b7 / b4 * 2) as i64;
        }
        let x1 = p / (1 << 8);
        let x1 = x1 * x1;
        let x1 = (x1 * 3038) / (1 << 16);
        let x2 = (-7357 * p) / (1 << 16);
        let p = p + (x1 + x2 + 3791) / ( 1 << 4);

        (p as f64) / 100.0
    }


}

fn spawn_sensor_read_task<I2C>(
    temp_tx: broadcast::Sender<f64>,
    press_tx: broadcast::Sender<f64>,
    mut i2c: I2C, 
    calib: Bmp180CalibrationData, 
    pressure_precision: PressurePrecision,
    period_ms: u64,
) 
where
    I2C: i2c::I2c + Send + 'static,
{
    let (oss, press_cmd, press_wait) = match pressure_precision {
        PressurePrecision::UltraLowPower => (0u8, MEASURE_PRESS_OSS_0, PRESS_0_WAIT),
        PressurePrecision::Standard => (1u8, MEASURE_PRESS_OSS_1, PRESS_1_WAIT),
        PressurePrecision::HighResolution => (2u8, MEASURE_PRESS_OSS_2, PRESS_2_WAIT),
        PressurePrecision::UltraHighResolution => (3u8, MEASURE_PRESS_OSS_3, PRESS_3_WAIT),
    };
    tokio::spawn(async move { 
        loop{
            let mut buffer: [u8; 2] = [0u8 ; 2];

            i2c.write(I2C_ADDRESS, &[CONTROL_REGISTER, MEASURE_TEMPERATURE]).unwrap();
            sleep(TEMP_WAIT).await;
            i2c.write_read(I2C_ADDRESS, &[OUT_MSB_REGISTER], &mut buffer).unwrap();
            let ut = ((buffer[0] as i32) << 8) | (buffer[1] as i32);

            let temp_res = calib.calc_temperature_c(ut);
            // info!("ut: {}, temp: {}", ut, temp_res.temp_c);
            if let Err(err) = temp_tx.send(temp_res.temp_c) {
                error!("error sending temperature data. shutting down bmp180 task. {:?}", err);
                break;
            }

            i2c.write(I2C_ADDRESS, &[CONTROL_REGISTER, press_cmd]).unwrap();
            sleep(press_wait).await;

            if oss == 3 {
                buffer = [0u8, 3];
            } else {
                buffer = [0u8, 2];
            }
            i2c.write_read(I2C_ADDRESS, &[OUT_MSB_REGISTER], &mut buffer).unwrap();
            let up: i32; 
            if oss == 3 {
                up = ( ((buffer[0] as i32) << 16) | ((buffer[1] as i32) << 8) | (buffer[2] as i32) ) >> (8 - oss);
            } else {
                up = ( ((buffer[0] as i32) << 16) | ((buffer[1] as i32) << 8) ) >> (8 - oss);
            }
            
            let press = calib.calc_pressure_h_pa(temp_res.b5, oss, up);
            //info!("up: {}, pressure: {}", up, press);
            if let Err(err) = press_tx.send(press) {
                error!("error sending pressure data. shutting down bmp180 task. {:?}", err);
                break;
            }

            let delay: tokio::time::Sleep = sleep(Duration::from_millis(period_ms));
            delay.await;
        }  
    });
}

impl Bmp180Device {

    pub fn build<I2C>(config: &Bmp180DeviceConfig, mut i2c: I2C) -> Result<Self, DeviceConfigError>
    where
        I2C: i2c::I2c + Send + 'static,
    {

        let mut buffer = [0u8];
        if let Err(err) = i2c.write_read(I2C_ADDRESS, &[ID_REGISTER], &mut buffer) {
            Err(DeviceConfigError::new(format!("Got error reading ID regsiter for BMP180 Pressure sensor. {:?}", err)))
        } else if buffer[0] != ID_EXPECTED_VALUE {
            Err(DeviceConfigError::new(format!("Expected to get {} when reading id register from BMP180 pressure sensor, but got a different value. There may be a different device connected.", ID_EXPECTED_VALUE)))
        } else {
            //read calibration data
            let mut buffer = [0u8 ; 22];
            i2c.write_read(I2C_ADDRESS, &[AC1_MSB_REGISTER], &mut buffer).unwrap();
            let calib = Bmp180CalibrationData::from_bytes(&buffer)?;
            // info!("calibration data!: {:?}", calib);

            let (temp_tx, temp_rx) = broadcast::channel(10);
            let (press_tx, press_rx) = broadcast::channel(10);

            spawn_sensor_read_task(temp_tx, press_tx, i2c, calib, config.pressure_precision, config.period_ms);

            Ok(Self {
                temperature_c: ScalerInput::new(temp_rx),
                pressure_h_pa: ScalerInput::new(press_rx),
            })
        }
    }
}

