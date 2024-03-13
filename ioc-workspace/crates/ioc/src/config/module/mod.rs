
//ioc_server 
#[cfg(feature = "server")]
use ioc_server::{Server, ServerConfig};

//ioc_extra
#[cfg(feature = "extra")]
use ioc_extra::input::noise::{NoiseInput, NoiseInputConfig};

//ioc_devices
#[cfg(feature = "devices")]
use ioc_devices::devices::pca9685::{Pca9685DeviceBuilder,Pca9685DeviceConfig};

//rpi (for i2c source)
#[cfg(feature = "rpi")]
use ioc_rpi_gpio::rppal::i2c::I2c;

use serde::Deserialize;
use ioc_core::{error::IocBuildError, Module, ModuleBuilder, ModuleIO};

#[cfg(feature = "rpi")]
fn i2c_bus_provider(bus: u8) -> I2c {
    ioc_rpi_gpio::get_bus(bus)
}

#[derive(Deserialize,Debug)]
pub enum IocModuleConfig {
    //ioc_server 
    #[cfg(feature = "server")]
    Server(ServerConfig),

    //ioc_extra 
    #[cfg(feature = "extra")]
    Noise(NoiseInputConfig),

    //ioc_devices
    #[cfg(feature = "devices")]
    Pca9685(Pca9685DeviceConfig)
}

impl IocModuleConfig {
    pub async fn build(&self) -> Result<ModuleIO, IocBuildError> {
       match self {
            #[cfg(feature = "server")]
            Self::Server(server_config) => {
                Server::try_build(&server_config).await.map(|server| server.into())
            }

            #[cfg(feature = "extra")]
            Self::Noise(noise_config) => {
                NoiseInput::try_build(&noise_config).await.map(|noise| noise.into())
            }

            #[cfg(feature = "devices")]
            Self::Pca9685(pca9685_config) => {
                Pca9685DeviceBuilder::new(i2c_bus_provider).try_build(pca9685_config).await.map(|outputs| outputs.into())
            }
        }
    }
}