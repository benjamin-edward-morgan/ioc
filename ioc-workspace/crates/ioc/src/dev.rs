use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,info};

use ioc_core::controller::IdentityController;
use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};
use ioc_extra::output::{console::ConsoleOutput, childproc::ChildProcessInput};
use std::collections::HashMap;

pub async fn dev_main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ioc=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let input_configs = HashMap::from([
        ("pan", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("tilt", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("fr", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("lr", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 2.0/2048.0 }),
        ("headlights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("taillights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
    ]);
    
    let output_configs = HashMap::from([]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec!["pan", "tilt", "fr", "lr", "headlights", "taillights"],
        outputs: vec![],
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "assets"
    };

    println!("build input");
    let mjpeg_in = ChildProcessInput::new();

    println!("build output");
    let mjpeg_config = EndpointConfig::Mjpeg { frames: mjpeg_in.rx };

    let cfg = ServerConfig{
        port: 8080,
        root_context: "/",
        inputs: input_configs,
        outputs: output_configs,
        endpoints: HashMap::from([
            ("/", static_endpoint_config),
            ("/ws", ws_endpoint_config),
            ("/stream", mjpeg_config),
        ]),
        state_channel_size: 5,
        io_channel_size: 5,
    };
       
    let server = Server::try_build(cfg).await.unwrap();
    println!("built server state!");
    // match (
    //     server.inputs.get("pan"),
    //     server.inputs.get("tilt"),
    //     server.inputs.get("fr"),
    //     server.inputs.get("lr"),
    // ) {
    //     (
    //         Some(TypedInput::Float(pan)),
    //         Some(TypedInput::Float(tilt)),
    //         Some(TypedInput::Float(fr)),
    //         Some(TypedInput::Float(lr)),
    //     ) => {
    //         let pan_out = ConsoleOutput::new("pan");
    //         let tilt_out = ConsoleOutput::new("tilt");
    //         let fr_out = ConsoleOutput::new("fr");
    //         let lr_out = ConsoleOutput::new("lr");


    //         let _idc0 = IdentityController::new(pan, &pan_out);
    //         let _idc1 = IdentityController::new(tilt, &tilt_out);
    //         let _idc2 = IdentityController::new(fr, &fr_out);
    //         let _idc3 = IdentityController::new(lr, &lr_out);  
    //     },
    //     x => {
    //         panic!("wrong!\n{:?}", x);
    //     }
    // }


    info!("started up!");
    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
