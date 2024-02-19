use ioc_core::channel::Channel;
use ioc_extra::controller::average::WindowedAverageValueController;
use ioc_extra::hw::{servo::ServoController, hbridge::HBridgeController};
use ioc_extra::output::childproc::ChildProcessInput;
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
        ("fr", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("lr", ServerInputConfig::Float{ start: 0.0, min: -1.0, max: 1.0, step: 2.0/2048.0 }),
        ("headlights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("taillights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
    ]);
    
    let output_configs = HashMap::from([]);

    let ws_endpoint_config = EndpointConfig::WebSocket {
        inputs: vec!["pan", "tilt", "fr", "lr", "headlights", "taillights"],
        outputs: vec![],
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "/home/beef/assets"
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




    let i2c = ioc_rpi_gpio::get_bus();
    let confg = Pca9685DeviceConfig{
        i2c_address: 64,
        channels: HashMap::from([
            ("servo0-pwm", 0),
            ("servo1-pwm", 1),
            ("headlights-pwm", 4),
            ("taillights-pwm", 5),
            ("a-enable-pwm", 10),
            ("a-fwd-pwm", 12),
            ("a-rev-pwm", 11),
            ("b-enable-pwm", 13),
            ("b-fwd-pwm", 14),
            ("b-rev-pwm", 15),
            
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
        server.inputs.get("fr"),
        server.inputs.get("lr"),
        server.inputs.get("taillights"),
        server.inputs.get("headlights"),
        pwm.channels.get("servo0-pwm"),
        pwm.channels.get("servo1-pwm"),
        pwm.channels.get("a-enable-pwm"),
        pwm.channels.get("a-fwd-pwm"),
        pwm.channels.get("a-rev-pwm"),
        pwm.channels.get("b-enable-pwm"),
        pwm.channels.get("b-fwd-pwm"),
        pwm.channels.get("b-rev-pwm"),
        pwm.channels.get("headlights-pwm"),
        pwm.channels.get("taillights-pwm"),
    ) {
        (
            Some(TypedInput::Float(pan)),
            Some(TypedInput::Float(tilt)),
            Some(TypedInput::Float(drive)),
            Some(TypedInput::Float(steer)),
            Some(TypedInput::Float(headlights)),
            Some(TypedInput::Float(taillights)),
            Some(pwm0_out),
            Some(pwm1_out),
            Some(pwm_a_enable),
            Some(pwm_a_fwd),
            Some(pwm_a_rev),
            Some(pwm_b_enable),
            Some(pwm_b_fwd),
            Some(pwm_b_rev),
            Some(pwm_headlights),
            Some(pwm_taillights),
        ) => {
          

            //hbridge controllers for steer and drive
            let drive_chan = Channel::new(0.0);
            let _drive_debounce = WindowedAverageValueController::new(drive, &drive_chan, 25);

            let steer_chan = Channel::new(0.0);
            let _steer_debounce = WindowedAverageValueController::new(steer, &steer_chan, 25);

            let _hbr0 = HBridgeController::new(&drive_chan, pwm_a_fwd, pwm_a_rev, pwm_a_enable).await;
            let _hbr1 = HBridgeController::new(&steer_chan, pwm_b_fwd, pwm_b_rev, pwm_b_enable).await;

            //servo controllers for pan and tilt
            let pan_chan = Channel::new(0.0);
            let _pan_debounce = WindowedAverageValueController::new(pan, &pan_chan, 25);

            let tilt_chan = Channel::new(0.0);
            let _lilt_debounce = WindowedAverageValueController::new(tilt, &tilt_chan, 25);

            let _pan = ServoController::new(&pan_chan, pwm0_out).await;
            let _tilt = ServoController::new(&tilt_chan, pwm1_out).await;

            //headlights, taillights 
            let _headlight_idc = IdentityController::new(headlights, pwm_headlights);
            let _taillight_idc = IdentityController::new(taillights, pwm_taillights);
            
        },
        _ => {
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
