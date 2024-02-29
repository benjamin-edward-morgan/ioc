use ioc_core::Input;

#[cfg(feature = "pca9685")]
pub mod pca9685;

#[cfg(feature = "lsm303agr")]
pub mod lsm303agr;

#[cfg(feature = "lsm303dlhc")]
pub mod lsm303dlhc;
