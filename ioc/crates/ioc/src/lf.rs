use ioc_core::channel::Channel;
use ioc_core::input::SumInput;
use ioc_core::{Input, Output};
use ioc_devices::devices::bmp180::{Bmp180DeviceConfig,Bmp180Device};
use ioc_devices::devices::l3gd20::{L3gd20Device, L3gd20DeviceConfig};
use ioc_extra::controller::average::WindowedAverageValueController;
use ioc_extra::hw::{servo::ServoController, hbridge::HBridgeController};
use ioc_extra::hw::camera::Camera;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing::{error,warn,info};
use ioc_core::controller::IdentityController;
use ioc_server::{Server, ServerConfig, EndpointConfig, ServerOutputConfig, ServerInputConfig, TypedInput, TypedOutput};
use ioc_devices::devices::pca9685::{Pca9685DeviceConfig, Pca9685Device};
use ioc_devices::devices::lsm303dlhc::Lsm303dlhcDevice;
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
        ("headlights", ServerInputConfig::Float{ start: 0.5, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("taillights", ServerInputConfig::Float{ start: 0.0, min: 0.0, max: 1.0, step: 1.0/2048.0 }),
        ("enable_camera", ServerInputConfig::Bool { start: true }),
    ]);
    
    let output_configs = HashMap::from([
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
            "pan", "tilt", "pan_trim", "tilt_trim",
             "drive", "steer", 
             "headlights", "taillights",
             "enable_camera" 
        ],
        outputs: vec![
            "accel_x","accel_y","accel_z",
            "mag_x", "mag_y", "mag_z",
            "gyro_x", "gyro_y", "gyro_z",
            "temperature", "pressure",
        ],
    };

    let static_endpoint_config = EndpointConfig::Static {
        directory: "/home/beef/assets"
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
            ("/stream", mjpeg_config),
        ]),
        state_channel_size: 5,
        io_channel_size: 5,
    };
       
    let server = Server::try_build(cfg).await.unwrap();



    let i2c = ioc_rpi_gpio::get_bus();
    let mag_accel = Lsm303dlhcDevice::new(i2c).unwrap();

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

            let mut rx = mag_accel.accelerometer.source().rx;

            tokio::spawn(async move {
                while let Ok((x,y,z)) = rx.recv().await {
                    tokio::join!(
                        x_out.send(x),
                        y_out.send(y),
                        z_out.send(z),
                    );
                }
                warn!("done publishing accel!");
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

            let mut rx = mag_accel.magnetometer.source().rx;

            tokio::spawn(async move {
                while let Ok((x,y,z)) = rx.recv().await {
                    tokio::join!(
                        x_out.send(x),
                        y_out.send(y),
                        z_out.send(z),
                    );
                }
                warn!("done publishing accel!");
            });
        },
        _ => panic!("wrong! unable to build accelerometer system")
    }

    let i2c = ioc_rpi_gpio::get_bus();
    let gyro = L3gd20Device::new(&L3gd20DeviceConfig::default(), i2c).unwrap();
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

            let mut rx = gyro.gyroscope.source().rx;

            tokio::spawn(async move {
                while let Ok((x,y,z)) = rx.recv().await {
                    tokio::join!(
                        x_out.send(x),
                        y_out.send(y),
                        z_out.send(z),
                    );
                }
                warn!("done publishing gyro!");
            });
        },
        _ => panic!("wrong! unable to build gyro system")
    }

    let i2c = ioc_rpi_gpio::get_bus();
    let temp_press = Bmp180Device::build(&Bmp180DeviceConfig::default(), i2c).unwrap();

    match (
        server.outputs.get("temperature"),
        server.outputs.get("pressure"),
    ) {
        (
            Some(TypedOutput::Float(temp_out)),
            Some(TypedOutput::Float(press_out)),
        ) => {
            let _ = IdentityController::new(&temp_press.temperature_c, temp_out);
            let _ = IdentityController::new(&temp_press.pressure_h_pa, press_out);
        },
        _ => {
            panic!("wrong! failed to build pressure/temp system");
        }
    }

    let i2c = ioc_rpi_gpio::get_bus();
    let confg = Pca9685DeviceConfig{
        i2c_address: 64,
        channels: HashMap::from([
            ("pan-servo-pwm", 0),
            ("tilt-servo-pwm", 1),
            ("taillights-pwm", 2),
            ("headlights-pwm", 3),
            ("drive-enable-pwm", 4),
            ("drive-rev-pwm", 5),
            ("drive-fwd-pwm", 6),
            ("steer-enable-pwm", 7),
            ("steer-left-pwm", 8),
            ("steer-right-pwm", 9),
        ])
    };
    let pwm = Pca9685Device::build(confg, i2c).unwrap();

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
