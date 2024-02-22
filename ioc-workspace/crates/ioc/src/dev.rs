use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,info};

use ioc_core::{controller::IdentityController, input::SumInput};
use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};
use ioc_extra::{hw::hbridge::HBridgeController, output::childproc::ChildProcessInput};
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
        ("pan_trim", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("tilt_trim", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("drive", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("steer", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("headlights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("taillights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
    ]);
    
    let output_configs = HashMap::from([
        ("pan_out", ServerOutputConfig::Float),
        ("tilt_out", ServerOutputConfig::Float),
        ("drive_fwd_out", ServerOutputConfig::Float),
        ("drive_rev_out", ServerOutputConfig::Float),
        ("drive_enable_out", ServerOutputConfig::Float),
        ("steer_left_out", ServerOutputConfig::Float),
        ("steer_right_out", ServerOutputConfig::Float),
        ("steer_enable_out", ServerOutputConfig::Float),
    ]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec!["pan", "tilt", "pan_trim", "tilt_trim", "drive", "steer", "headlights", "taillights"],
        outputs: vec![],
    };

    let debug_ws_endpoint_config = EndpointConfig::WebSocket { 
        inputs: vec![], 
        outputs: vec!["pan_out", "tilt_out", "drive_fwd_out", "drive_rev_out", "drive_enable_out", "steer_left_out", "steer_right_out", "steer_enable_out"] 
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
            ("/debug", debug_ws_endpoint_config),
            ("/stream", mjpeg_config),
        ]),
        state_channel_size: 5,
        io_channel_size: 5,
    };
       
    let server = Server::try_build(cfg).await.unwrap();
    println!("built server state!");

    match (
        server.inputs.get("pan"),
        server.inputs.get("tilt"),
        server.inputs.get("pan_trim"),
        server.inputs.get("tilt_trim"),
        server.outputs.get("pan_out"),
        server.outputs.get("tilt_out"),
    ) {
        (
            Some(TypedInput::Float(pan)),
            Some(TypedInput::Float(tilt)),
            Some(TypedInput::Float(pan_trim)),
            Some(TypedInput::Float(tilt_trim)),
            Some(TypedOutput::Float(pan_out)),
            Some(TypedOutput::Float(tilt_out)),   
        ) => {
            let pan_sum = SumInput::new(20, vec![pan, pan_trim]);
            let tilt_sum = SumInput::new(20, vec![tilt, tilt_trim]);

            let _idc2 = IdentityController::new(&pan_sum, pan_out);
            let _idc3 = IdentityController::new(&tilt_sum, tilt_out);  
        },
        _x => {
            panic!("wrong! failed to build camera pan/tilt system!");
        }
    }

    match (
        server.inputs.get("drive"),
        server.inputs.get("steer"),
        server.outputs.get("drive_fwd_out"), 
        server.outputs.get("drive_rev_out"), 
        server.outputs.get("drive_enable_out"), 
        server.outputs.get("steer_left_out"), 
        server.outputs.get("steer_right_out"), 
        server.outputs.get("steer_enable_out"),
    ) {
        (
            Some(TypedInput::Float(drive)),
            Some(TypedInput::Float(steer)),
            Some(TypedOutput::Float(drive_fwd_out)),
            Some(TypedOutput::Float(drive_rev_out)),
            Some(TypedOutput::Float(drive_enable_out)),
            Some(TypedOutput::Float(steer_left_out)),
            Some(TypedOutput::Float(steer_right_out)),
            Some(TypedOutput::Float(steer_enable_out)),
        ) => {
            let _drive_hbridge = HBridgeController::new(drive, drive_fwd_out, drive_rev_out, drive_enable_out);
            let _steer_hbridge = HBridgeController::new(steer, steer_left_out, steer_right_out, steer_enable_out);     
        },
        _ => {
            panic!("wrong! failed to build steering/driving system!")
        },
    }


    info!("started up!");
    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
