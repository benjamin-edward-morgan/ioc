pub mod pca9685;


use pca9685::Pca9685DeviceConfig;



pub struct I2CDeviceConfigError {
    message: String
}



pub enum SystemDevicesConfig {
    Pca9685(Pca9685DeviceConfig),
}

