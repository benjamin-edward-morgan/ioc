
pub mod input;
pub mod output;
pub mod error;

use crate::error::BuildError;
use rppal::gpio::Gpio;


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
