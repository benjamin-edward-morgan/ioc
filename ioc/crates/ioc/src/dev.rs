use ioc_extra::hw::camera::Camera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,info};
use tokio::time::{sleep, Duration};
use ioc_core::{Output, controller::IdentityController, input::SumInput, channel::Channel};
use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};
use ioc_extra::hw::hbridge::HBridgeController;
use ioc_extra::controller::average::WindowedAverageValueController;
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
        ("enable_camera", ServerInputConfig::Bool { start: true })
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
        ("headlights_out", ServerOutputConfig::Float),
        ("taillights_out", ServerOutputConfig::Float),  

        ("accel_x", ServerOutputConfig::Float),
        ("accel_y", ServerOutputConfig::Float),
        ("accel_z", ServerOutputConfig::Float),
        ("mag_x", ServerOutputConfig::Float),
        ("mag_y", ServerOutputConfig::Float),
        ("mag_z", ServerOutputConfig::Float),  
        ("gyro_x", ServerOutputConfig::Float),
        ("gyro_y", ServerOutputConfig::Float),
        ("gyro_z", ServerOutputConfig::Float),  
        ("temperature", ServerOutputConfig::Float),
        ("pressure", ServerOutputConfig::Float),
    ]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec![
            "pan", "tilt", 
            "pan_trim", "tilt_trim", 
            "drive", "steer", 
            "headlights", "taillights",
            "enable_camera",
        ],
        outputs: vec![
            "accel_x","accel_y","accel_z",
            "mag_x", "mag_y", "mag_z",
            "gyro_x", "gyro_y", "gyro_z",
            "temperature", "pressure"
        ],
    };

    let debug_ws_endpoint_config = EndpointConfig::WebSocket { 
        inputs: vec![], 
        outputs: vec![
            "pan_out", "tilt_out", 
            "drive_fwd_out", "drive_rev_out", "drive_enable_out", 
            "steer_left_out", "steer_right_out", "steer_enable_out",
            "headlights_out", "taillights_out"
        ] 
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "assets"
    };

    info!("building camera mjpeg stream...");
    let cam_enable_chan = Channel::new(true);
    let cam = Camera::new(&cam_enable_chan);
    
    info!("building camera mjpeg endpoint...");
    let mjpeg_config = EndpointConfig::Mjpeg { frames: cam.mjpeg };

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
            let pan_chan = Channel::new(0.0);
            let _pan_windows = WindowedAverageValueController::new(&pan_sum, &pan_chan, 25);


            let tilt_sum = SumInput::new(20, vec![tilt, tilt_trim]);
            let tilt_chan = Channel::new(0.0);
            let _tilt_windows = WindowedAverageValueController::new(&tilt_sum, &tilt_chan, 25);

            let _idc2 = IdentityController::new(&pan_chan, pan_out);
            let _idc3 = IdentityController::new(&tilt_chan, tilt_out);  
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

            let drive_chan = Channel::new(0.0);
            let _drive_windows = WindowedAverageValueController::new(drive, &drive_chan, 25);

            let steer_chan = Channel::new(0.0);
            let _steer_windows = WindowedAverageValueController::new(steer, &steer_chan, 25);

            let _drive_hbridge = HBridgeController::new(&drive_chan, drive_fwd_out, drive_rev_out, drive_enable_out);
            let _steer_hbridge = HBridgeController::new(&steer_chan, steer_left_out, steer_right_out, steer_enable_out);     
        },
        _ => {
            panic!("wrong! failed to build steering/driving system!")
        },
    }

    match (
        server.inputs.get("headlights"),
        server.inputs.get("taillights"),
        server.outputs.get("headlights_out"),
        server.outputs.get("taillights_out"),
    ) {
        (
            Some(TypedInput::Float(headlights)),
            Some(TypedInput::Float(taillights)),
            Some(TypedOutput::Float(headlights_out)),
            Some(TypedOutput::Float(taillights_out)),

        ) => {
            let headl_chan = Channel::new(0.0);
            let _headl_debounce = WindowedAverageValueController::new(headlights, &headl_chan, 25);

            let taill_chan = Channel::new(0.0);
            let _taill_debounce = WindowedAverageValueController::new(taillights, &taill_chan, 25);

            let _hl_idc = IdentityController::new(&headl_chan, headlights_out);
            let _tl_idc = IdentityController::new(&taill_chan, taillights_out);
        },
        _ => {
            panic!("wrong! failed to build lights system!");
        }
    }

    match (
        server.inputs.get("enable_camera"),
    ) {
        (Some(TypedInput::Bool(enable_camera)),) => {
            let _ = IdentityController::new(enable_camera, &cam_enable_chan);
        },
        _ => {
            panic!("wrong! unable to build camera controls!");
        }
    }


    match (
        server.outputs.get("accel_x"),
        server.outputs.get("accel_y"),
        server.outputs.get("accel_z"),
    ) {
        (
            Some(TypedOutput::Float(accel_x)),
            Some(TypedOutput::Float(accel_y)),
            Some(TypedOutput::Float(accel_z)),   
        ) => {
            let x_out = accel_x.sink().tx;
            let y_out = accel_y.sink().tx;
            let z_out = accel_z.sink().tx;

            tokio::spawn(async move {
                loop {
                    tokio::join!(
                        x_out.send(rand::random::<f64>()-0.5),
                        y_out.send(rand::random::<f64>()-0.5),
                        z_out.send(rand::random::<f64>()-0.5),
                        sleep(Duration::from_millis(1000)),
                    );
                }
            });
        },
        _ => panic!("wrong! unable to build accelerometer system")
    }

    match (
        server.outputs.get("mag_x"),
        server.outputs.get("mag_y"),
        server.outputs.get("mag_z"),
    ) {
        (
            Some(TypedOutput::Float(mag_x)),
            Some(TypedOutput::Float(mag_y)),
            Some(TypedOutput::Float(mag_z)),   
        ) => {
            let x_out = mag_x.sink().tx;
            let y_out = mag_y.sink().tx;
            let z_out = mag_z.sink().tx;


            tokio::spawn(async move {
                loop {
                    tokio::join!(
                        x_out.send((rand::random::<f64>()-0.5)*0.04),
                        y_out.send((rand::random::<f64>()-0.5)*0.04),
                        z_out.send((rand::random::<f64>()-0.5)*0.04),
                        sleep(Duration::from_millis(1000)),
                    );
                }
            });
        },
        _ => panic!("wrong! unable to build magnetometer system")
    }

    match (
        server.outputs.get("gyro_x"),
        server.outputs.get("gyro_y"),
        server.outputs.get("gyro_z"),
    ) {
        (
            Some(TypedOutput::Float(gyro_x)),
            Some(TypedOutput::Float(gyro_y)),
            Some(TypedOutput::Float(gyro_z)),   
        ) => {
            let x_out = gyro_x.sink().tx;
            let y_out = gyro_y.sink().tx;
            let z_out = gyro_z.sink().tx;


            tokio::spawn(async move {
                loop {
                    tokio::join!(
                        x_out.send((rand::random::<f64>()-0.5)*40.0),
                        y_out.send((rand::random::<f64>()-0.5)*40.0),
                        z_out.send((rand::random::<f64>()-0.5)*40.0),
                        sleep(Duration::from_millis(1000)),
                    );
                }
            });
        },
        _ => panic!("wrong! unable to build gyro system")
    }

    match (
        server.outputs.get("temperature"),
        server.outputs.get("pressure"),
    ) {
        (
            Some(TypedOutput::Float(temperature)),
            Some(TypedOutput::Float(pressure)),  
        ) => {
            let t_out = temperature.sink().tx;
            let p_out = pressure.sink().tx;

            tokio::spawn(async move {
                loop {
                    tokio::join!(
                        t_out.send(22.0 + rand::random::<f64>()-0.5),
                        p_out.send(993.2 + rand::random::<f64>()-0.5),
                        sleep(Duration::from_millis(10000)),
                    );
                }
            });
        },
        _ => panic!("wrong! unable to build accelerometer system")
    }


    info!("started up!");
    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
