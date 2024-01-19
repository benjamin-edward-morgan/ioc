use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

use ioc_server::{Server, ServerConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();



    let server_builder = ServerBuilder::try_from(&ServerConfig{port: 8080}).unwrap();

    server_builder.add_endpoint("/", EndpointConfig::Static{ port: 8080, dir: "/foo/bar" });

    server_builder.add_endpoint("/ws", EndpointConfig::Websocket{ 
        inputs: HashMap("asdf" -> WebsocketInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 0.01} ),
        outouts: HashMap("qwerty" -> WebsocketOutputConfig::Float),
    }).unwrap();

    let server = server_builder.try_build().unwrap();

    server.handle.await
}
