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
        ("ws_float_in", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 0.01 }),
        ("ws_bool_in", ServerInputConfig::Bool{ start: false }),
        ("ws_string_in", ServerInputConfig::String{ start: "".to_string(), max_length: 16 }),
    ]);

    let output_configs = HashMap::from([
        ("ws_float_out", ServerOutputConfig::Float),
        ("ws_bool_out", ServerOutputConfig::Bool),
        ("ws_string_out", ServerOutputConfig::String),
    ]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec!["ws_float_in", "ws_bool_in", "ws_string_in"],
        outputs: vec!["ws_float_out", "ws_bool_out", "ws_string_out"],
    };

    // let static_endpoint_config = EndpointConfig::Static {
    //     directory: "assets"
    // };

    let cfg = ServerConfig{
        port: 8080,
        root_context: "/",
        inputs: input_configs,
        outputs: output_configs,
        endpoints: HashMap::from([
            // ("/", static_endpoint_config),
            ("/ws", ws_endpoint_config),
        ]),
        state_channel_size: 100,
    };
       
    let server = Server::try_build(cfg).await.unwrap();

    match (
        server.inputs.get("ws_float_in"),
        server.inputs.get("ws_bool_in"),
        server.inputs.get("ws_string_in"),
        server.outputs.get("ws_float_out"),
        server.outputs.get("ws_bool_out"),
        server.outputs.get("ws_string_out"),
    ) {
        (
            Some(TypedInput::Float(ws_float_in)),
            Some(TypedInput::Bool(ws_bool_in)),
            Some(TypedInput::String(ws_string_in)),
            Some(TypedOutput::Float(ws_float_out)),
            Some(TypedOutput::Bool(ws_bool_out)),
            Some(TypedOutput::String(ws_string_out)),
        ) => {
            let idc0 = IdentityController::new(ws_float_in, ws_float_out);
            let idc1 = IdentityController::new(ws_bool_in, ws_bool_out);
            let idc2 = IdentityController::new(ws_string_in, ws_string_out);
            
        },
        (
            _, _, _, _, _, _,
        ) => {
            panic!("wrong!");
        }
    }


    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
