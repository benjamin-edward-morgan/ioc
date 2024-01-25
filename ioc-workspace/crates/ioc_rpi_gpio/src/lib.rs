
pub mod input;
pub mod output;
pub mod error;

use crate::error::BuildError;
pub use rppal;
use rppal::{gpio::Gpio, i2c::I2c};



pub struct RpiGpioConfig {
    pub channel_size: u16,
}

pub struct RpiGpio {
    channel_size: u16,
    gpio: Gpio,
}

impl RpiGpio {
    pub fn try_build(cfg: &RpiGpioConfig) -> Result<Self, BuildError> {
        let gpio = Gpio::new()?;

        Ok(RpiGpio{
            channel_size: cfg.channel_size,
            gpio,
        })
    }
}


pub fn get_bus() -> I2c {
    I2c::new().unwrap()
}
