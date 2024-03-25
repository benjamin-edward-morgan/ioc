use ioc_core::{error::IocBuildError, feedback::{Feedback, FeedbackConfig}, Module, ModuleIO};
use serde::Deserialize;

//ioc_server
#[cfg(feature = "server")]
use ioc_server::{Server, ServerConfig};

//ioc_extra
#[cfg(feature = "extra")]
use ioc_extra::hw::camera::{Camera, CameraConfig};

//ioc_devices
#[cfg(feature = "devices")]
use ioc_devices::devices::{
    bmp180::{Bmp180DeviceBuilder, Bmp180DeviceConfig},
    l3gd20::{L3gd20DeviceBuilder, L3gd20DeviceConfig},
    lsm303dlhc::{Lsm303dlhcDeviceBuilder, Lsm303dlhcDeviceConfig},
    pca9685::{Pca9685DeviceBuilder, Pca9685DeviceConfig},
};
#[cfg(feature = "devices")]
use ioc_core::ModuleBuilder;


//rpi (for i2c source)
#[cfg(feature = "rpi")]
use ioc_rpi_gpio::{I2c, gpio::{Gpio, GpioConfig}};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "rpi")]
fn i2c_bus_provider(bus: u8) -> I2c {
    ioc_rpi_gpio::get_bus(bus)
}

/// Modules are collections of Inputs and/or Outputs provided by some black-box system.
#[derive(Deserialize, Debug)]
pub enum IocModuleConfig {
    //core 
    Feedback(FeedbackConfig),

    //ioc_server
    #[cfg(feature = "server")]
    Server(ServerConfig),

    //ioc_extra
    #[cfg(feature = "extra")]
    RaspiCam(CameraConfig),

    //ioc_devices
    #[cfg(feature = "devices")]
    Pca9685(Pca9685DeviceConfig),
    #[cfg(feature = "devices")]
    Bmp180(Bmp180DeviceConfig),
    #[cfg(feature = "devices")]
    L3dg20(L3gd20DeviceConfig),
    #[cfg(feature = "devices")]
    Lsm303dlhc(Lsm303dlhcDeviceConfig),

    //rpi 
    #[cfg(feature = "rpi")]
    Gpio(GpioConfig),
}

impl IocModuleConfig {
    pub async fn build(&self, cancel_token: CancellationToken) -> Result<ModuleIO, IocBuildError> {
        match self {
            //core
            Self::Feedback(feedback_config) => Feedback::try_build(feedback_config, cancel_token)
            .await
            .map(|feedback| feedback.into()),

            //server
            #[cfg(feature = "server")]
            Self::Server(server_config) => Server::try_build(server_config, cancel_token)
                .await
                .map(|server| server.into()),

            //extra
            #[cfg(feature = "extra")]
            Self::RaspiCam(cam_config) => Camera::try_build(cam_config, cancel_token).await.map(|cam| cam.into()),

            //devices
            #[cfg(feature = "devices")]
            Self::Pca9685(pca9685_config) => Pca9685DeviceBuilder::new(i2c_bus_provider)
                .try_build(pca9685_config)
                .await
                .map(|outputs| outputs.into()),
            #[cfg(feature = "devices")]
            Self::Bmp180(bmp180_config) => Bmp180DeviceBuilder::new(i2c_bus_provider)
                .try_build(bmp180_config)
                .await
                .map(|sensor| sensor.into()),
            #[cfg(feature = "devices")]
            Self::L3dg20(l3dg20_cfg) => L3gd20DeviceBuilder::new(i2c_bus_provider)
                .try_build(l3dg20_cfg)
                .await
                .map(|sensor| sensor.into()),
            #[cfg(feature = "devices")]
            Self::Lsm303dlhc(lsm303dlhc_cfg) => Lsm303dlhcDeviceBuilder::new(i2c_bus_provider)
                .try_build(lsm303dlhc_cfg)
                .await
                .map(|sensor| sensor.into()),
            
            //rpi
            #[cfg(feature = "rpi")]
            Self::Gpio(gpio_config) => Gpio::try_build(gpio_config, cancel_token).await.map(|gpio| gpio.into()),
        }
    }
}
