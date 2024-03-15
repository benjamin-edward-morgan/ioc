
use crate::{Output,OutputSink};
use linux_embedded_hal::I2cdev;
use pwm_pca9685::{Address, Pca9685};

//system level confic -- corresponds to 1 pwm chip instance 
#[derive(Debug)]
pub struct Pca9685DeviceConfig {
    i2c_device: u8,
    i2c_address: u8,
}


//connected pwm chip instance
pub struct Pca9685Device {

}

impl Pca9685Device {
    pub fn try_build_output(config: Pca9685PwmOutputConfig) -> Pca9685PwmOutput {
        todo!()
    }
}

impl TryFrom<Pca9685DeviceConfig> for Pca9685Device {
    type Error = I2CDeviceConfigError;
    fn try_from(value: Pca9685DeviceConfig) -> Result<Self, Self::Error> {
        todo!()
    }
}

//io level config -- corresponds to one pin on pwm chip instance
#[derive(Debug)]
pub struct Pca9685PwmOutputConfig {
    device: String,
    channel: u8,
}

//pwm float output associated with a channel (pin) on a Pca9685Device
pub struct Pca9685PwmOutput {

}

impl Output<f64> for Pca9685PwmOutput {
    fn sink(&self) -> OutputSink<f64> {
        todo!()
    }
}





