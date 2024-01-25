use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,info};
// use ioc_core::controller::IdentityController;

// use ioc_rpi_gpio::{RpiGpio,RpiGpioConfig};
// use ioc_rpi_gpio::input::digital_bool::{DigitalBoolInput, DigitalBoolInputConfig};
// use ioc_rpi_gpio::output::digital_bool::{DigitalBoolOutput, DigitalBoolOutputConfig};

// #[tokio::main]
// async fn main() {
//     tracing_subscriber::registry()
//         .with(
//             tracing_subscriber::EnvFilter::try_from_default_env()
//                 .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
//         )
//         .with(tracing_subscriber::fmt::layer())
//         .init();



//     let rpi_gpio = RpiGpio::try_build(&RpiGpioConfig{ channel_size: 16 }).unwrap();

//     let input = DigitalBoolInput::try_build(&rpi_gpio, &DigitalBoolInputConfig{ pin: 17, pull_up: true }).unwrap();
//     let output = DigitalBoolOutput::try_build(&rpi_gpio, &DigitalBoolOutputConfig{ pin: 23}).unwrap();

//     let controller = IdentityController::new(&input, &output);

//     controller.handle.await.unwrap()
// }


use ioc_core::controller::IdentityController;

use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};

use ioc_devices::devices::pca9685::{Pca9685DeviceConfig, Pca9685Device};
use ioc_devices::devices::lsm303::{Lsm303DeviceConfig, Lsm303Device};
use std::collections::HashMap;




#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let input_configs = HashMap::from([
        ("servo0", ServerInputConfig::Float{ start: 0.0, min: 0.00, max: 0.14, step: 0.001 }),
        ("servo1", ServerInputConfig::Float{ start: 0.0, min: 0.00, max: 0.14, step: 0.001 }),
    ]);

    let output_configs = HashMap::from([]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec!["servo0", "servo1"],
        outputs: vec![],
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "/home/beef/assets"
    };

    let cfg = ServerConfig{
        port: 80,
        root_context: "/",
        inputs: input_configs,
        outputs: output_configs,
        endpoints: HashMap::from([
            ("/", static_endpoint_config),
            ("/ws", ws_endpoint_config),
        ]),
        state_channel_size: 100,
    };
       
    let server = Server::try_build(cfg).await.unwrap();




    let i2c = ioc_rpi_gpio::get_bus();
    let confg = Pca9685DeviceConfig{
        i2c_address: 64,
        channels: HashMap::from([
            ("servo0-pwm", 0),
            ("servo1-pwm", 1)
        ])
    };
    let pwm = Pca9685Device::build(confg, i2c).unwrap();

    // let i2c = ioc_rpi_gpio::get_bus();
    // let confg = Lsm303DeviceConfig{

    // };
    // let mag_accel = Lsm303Device::build(confg, i2c).unwrap();

    match (
        server.inputs.get("servo0"),
        server.inputs.get("servo1"),
        pwm.channels.get("servo0-pwm"),
        pwm.channels.get("servo1-pwm"),
    ) {
        (
            Some(TypedInput::Float(posn0)),
            Some(TypedInput::Float(posn1)),
            Some(pwm0_out),
            Some(pwm1_out),
        ) => {
            let _idc0 = IdentityController::new(posn0, pwm0_out);
            let _idc1 = IdentityController::new(posn1, pwm1_out);
        },
        (
            _, _, _, _ ,
        ) => {
            panic!("wrong!");
        }
    }


    info!("started up!");
    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
