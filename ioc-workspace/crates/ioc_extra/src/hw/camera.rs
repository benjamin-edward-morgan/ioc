mod image;
mod child_process_stream;
mod jpeg_stream_splitter;

use std::ops::Deref;

use futures::Future;
use ioc_core::Input;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use image::TestPatternGenerator;
use tracing::info;
use child_process_stream::start_child_process;
use jpeg_stream_splitter::split_jpegs;

use self::{child_process_stream::ChildProcessError, image::JpegImage};

enum CameraState {
    MjpegStream()
}

pub struct Camera {
    pub mjpeg: watch::Receiver<Vec<u8>>,
}

// kill_switch: impl Future<Output = ()> + Send + 'static

fn start_mjpeg_stream(kill_switch: impl Future<Output = ()> + Send + 'static) -> Result<watch::Receiver<Option<JpegImage>>, ChildProcessError> {

    let args = [
        "--rotation", "180",
        "--width", "640",
        "--height","480",
        "--codec", "mjpeg",
        "--framerate", "5",
        "--tuning-file", "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
        "-q", "50",
        "-t", "0",
        "-n",
        "--flush",
        "-o", "-"
    ];

    start_child_process(
        "libcamera-vid", 
        &args, 
        split_jpegs,
        kill_switch,
    )

}

impl Camera {

    pub fn new(enable: &dyn Input<bool>) -> Self {

        let tp_gen = TestPatternGenerator::new(640, 480, 50);
        let tp = tp_gen.generate();
        info!("test pattern bytes {}", tp.bytes.len());

        let (frame_tx, frame_rx) = watch::channel(tp.bytes.clone());
        let mut enable_source = enable.source();
        let process_cancel_token = CancellationToken::new();
        
        let kill_switch = process_cancel_token.clone().cancelled_owned();
        tokio::spawn(async move {

            frame_tx.send(tp.bytes.clone()).expect("Error sending camera frame");

            let mut frames = start_mjpeg_stream(kill_switch).unwrap();

            info!("sending stream frames!");
            while let Ok(_) = frames.changed().await {
                let last_frame = frames.borrow();
                if let Some(lf) = last_frame.deref() {
                    frame_tx.send(lf.bytes.clone()).expect("Error sending camera frame");
                }
            }
            info!("camera mjpeg stream shut down");

            frame_tx.send(tp.bytes.clone()).expect("Error sending camera frame");

        });

        tokio::spawn(async move {
            while let Ok(enable) = enable_source.rx.recv().await {
                if !enable {
                    info!("stopping stream!");
                    process_cancel_token.cancel();
                    break;
                }
            }
        });

        Self {
            mjpeg: frame_rx
        }
    }
}