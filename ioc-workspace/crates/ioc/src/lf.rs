use ioc_core::channel::Channel;
use ioc_core::input::SumInput;
use ioc_extra::controller::average::WindowedAverageValueController;
use ioc_extra::hw::{servo::ServoController, hbridge::HBridgeController};
use ioc_extra::output::childproc::ChildProcessInput;
use ioc_extra::hw::camera::Camera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,info};
use ioc_core::controller::IdentityController;
use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};
use ioc_devices::devices::pca9685::{Pca9685DeviceConfig, Pca9685Device};
use ioc_devices::devices::lsm303::{Lsm303DeviceConfig, Lsm303Device};
use std::collections::HashMap;

pub async fn littlefoot_main() {
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
        ("pan_trim", ServerInputConfig::Float{ start: 0.0625, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("tilt_trim", ServerInputConfig::Float{ start: 0.7138671875, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("drive", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("steer", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("headlights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("taillights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("enable_camera", ServerInputConfig::Bool { start: true }),
    ]);
    
    let output_configs = HashMap::from([]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec![
            "pan", "tilt", "pan_trim", "tilt_trim",
             "drive", "steer", 
             "headlights", "taillights",
             "enable_camera" 
        ],
        outputs: vec![],
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "/home/beef/assets"
    };

    info!("building camera mjpeg stream...");
    // let mjpeg_in = ChildProcessInput::new();
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
            ("/stream", mjpeg_config),
        ]),
        state_channel_size: 5,
        io_channel_size: 5,
    };
       
    let server = Server::try_build(cfg).await.unwrap();

    let i2c = ioc_rpi_gpio::get_bus();
    let confg = Pca9685DeviceConfig{
        i2c_address: 64,
        channels: HashMap::from([
            ("pan-servo-pwm", 0),
            ("tilt-servo-pwm", 1),
            ("taillights-pwm", 4),
            ("headlights-pwm", 5),
            ("drive-enable-pwm", 10),
            ("drive-fwd-pwm", 12),
            ("drive-rev-pwm", 11),
            ("steer-enable-pwm", 13),
            ("steer-left-pwm", 14),
            ("steer-right-pwm", 15),
        ])
    };
    let pwm = Pca9685Device::build(confg, i2c).unwrap();

    // let i2c = ioc_rpi_gpio::get_bus();
    // let confg = Lsm303DeviceConfig{

    // };
    // let mag_accel = Lsm303Device::build(confg, i2c).unwrap();

    match (
        server.inputs.get("pan"),
        server.inputs.get("tilt"),
        server.inputs.get("pan_trim"),
        server.inputs.get("tilt_trim"),
        pwm.channels.get("pan-servo-pwm"),
        pwm.channels.get("tilt-servo-pwm"),
    ) {
        (
            Some(TypedInput::Float(pan)),
            Some(TypedInput::Float(tilt)),
            Some(TypedInput::Float(pan_trim)),
            Some(TypedInput::Float(tilt_trim)),
            Some(pan_out),
            Some(tilt_out),            
        ) => {
            //servo controllers for pan and tilt
            let pan_sum = SumInput::new(50, vec![pan, pan_trim]);
            let pan_chan = Channel::new(0.0);
            let _pan_windows = WindowedAverageValueController::new(&pan_sum, &pan_chan, 25);

            let tilt_sum = SumInput::new(50, vec![tilt, tilt_trim]);
            let tilt_chan = Channel::new(0.0);
            let _tilt_windows = WindowedAverageValueController::new(&tilt_sum, &tilt_chan, 25);

            let _pan = ServoController::new(&pan_chan, pan_out).await;
            let _tilt = ServoController::new(&tilt_chan, tilt_out).await;
        },
        _ => {
            panic!("wrong! failed to build pan-tilt system");
        }
    }

    match (
        server.inputs.get("drive"),
        server.inputs.get("steer"),
        pwm.channels.get("drive-enable-pwm"),
        pwm.channels.get("drive-fwd-pwm"),
        pwm.channels.get("drive-rev-pwm"),
        pwm.channels.get("steer-enable-pwm"),
        pwm.channels.get("steer-left-pwm"),
        pwm.channels.get("steer-right-pwm"),
    ) {
        (
            Some(TypedInput::Float(drive)),
            Some(TypedInput::Float(steer)),
            Some(drive_enable_pwm),
            Some(drive_fwd_pwm),
            Some(drive_rev_pwm),
            Some(steer_rev_pwm),
            Some(steer_left_pwm),
            Some(steer_right_pwm),
        ) => {

            //hbridge controllers for steer and drive
            let drive_chan = Channel::new(0.0);
            let _drive_debounce = WindowedAverageValueController::new(drive, &drive_chan, 25);

            let steer_chan = Channel::new(0.0);
            let _steer_debounce = WindowedAverageValueController::new(steer, &steer_chan, 25);

            let _hbr0 = HBridgeController::new(&drive_chan, drive_fwd_pwm, drive_rev_pwm, drive_enable_pwm);
            let _hbr1 = HBridgeController::new(&steer_chan, steer_left_pwm, steer_right_pwm, steer_rev_pwm);

        },
        _ => {
            panic!("wrong! failed to build drive-steer system");
        }
    }

    match (
        server.inputs.get("taillights"),
        server.inputs.get("headlights"),
        pwm.channels.get("headlights-pwm"),
        pwm.channels.get("taillights-pwm"),
    ) {
        (
            Some(TypedInput::Float(headlights)),
            Some(TypedInput::Float(taillights)),
            Some(pwm_headlights),
            Some(pwm_taillights),
        ) => {
            let hl_chan = Channel::new(0.0);
            let _hl_debounce = WindowedAverageValueController::new(headlights, &hl_chan, 25);

            let tl_chan = Channel::new(0.0);
            let _tl_debounce = WindowedAverageValueController::new(taillights, &tl_chan, 25);

            let _headlight_idc = IdentityController::new(&hl_chan, pwm_headlights);
            let _taillight_idc = IdentityController::new(&tl_chan, pwm_taillights);
        },
        _ => {
            panic!("wrong! failed to build lights system");
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


    info!("started up!");
    if let Err(err) = server.handle.await {
        error!("Server is exiting unsuccessfully :(\n{:?}", err);
    } else {
        info!("Server is exiting.")
    }
    
}
