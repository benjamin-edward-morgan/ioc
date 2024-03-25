//!This library provides access to the Raspberry Pi GPIO pins. It is a wrapper around the rppal library.
//! 
//! The `get_bus` and `get_default_bus` functions can get an I2C bus instance that can be used to construct modules in `ioc_devices`

//internal error type for rpi gpio
pub mod error;

//module to use bare gpio pins as inputs and outputs
pub mod gpio;

pub use rppal::i2c::I2c;

//get i2c bus by id
pub fn get_bus(bus: u8) -> I2c {
    I2c::with_bus(bus).unwrap()
}

//get default i2c bus
pub fn get_default_bus() -> I2c {
    I2c::new().unwrap()
}
