

///The PCA9685 is an i2c device from NXP with 16 pwm output channels with 12 bit resolution.
#[cfg(feature = "pca9685")]
pub mod pca9685;

///The LSM303DLHC is an i2c device from STMicroelectronics. It includes a MEMS accelerometer and a MEMS magnetometer.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "lsm303dlhc")]
pub mod lsm303dlhc;

///The L3GD20 is an i2c device from STMicroelectronics with a MEMS gyroscope sensor.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "l3gd20")]
pub mod l3gd20;

///The BMP180 is an i2c device from Bosch Sensortec. It includes an ambient temperature sensor and an atmospheric pressure senso.
///
/// It is discontinued at the time of writing, but they still exist and can be useful.
#[cfg(feature = "bmp180")]
pub mod bmp180;
