mod image;
mod child_process_stream;
mod jpeg_stream_splitter;

use std::{ops::Deref, sync::{Arc, Mutex}};
use futures::Future;
use ioc_core::Input;
use tokio::{sync::{broadcast, oneshot, watch}, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use image::TestPatternGenerator;
use tracing::{info, warn};
use child_process_stream::start_child_process;
use jpeg_stream_splitter::split_jpegs;
use self::{child_process_stream::ChildProcessError, image::JpegImage};

struct MjpegStreamState {
    pub callback: oneshot::Receiver<watch::Sender<Vec<u8>>>,
    pub enable_rx: broadcast::Receiver<bool>,
}

struct StreamEndedState {
    pub frame_tx: watch::Sender<Vec<u8>>,
    pub enable_rx: broadcast::Receiver<bool>,
}

enum CameraState {
    MjpegStream(MjpegStreamState),
    StreamEnded(StreamEndedState),
}

impl CameraState {
    async fn step(self) -> Self {
        match self {
            Self::MjpegStream(stream) => {
                if let Ok(frame_tx) = stream.callback.await {
                    let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
                    frame_tx.send(tp).unwrap();
                    Self::StreamEnded(StreamEndedState{frame_tx: frame_tx, enable_rx: stream.enable_rx.resubscribe()})
                } else {
                    panic!("did not get frame_tx from callback :(");
                }
            },
            Self::StreamEnded(mut ended) => {
                let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
                ended.frame_tx.send(tp.clone()).unwrap();
                if let Ok(is_enabled) = ended.enable_rx.recv().await {
                    if is_enabled {
                        Self::MjpegStream(spawn_camera(ended.frame_tx, ended.enable_rx))
                    } else {
                        Self::StreamEnded(ended)
                    }
                } else {
                    panic!("enable_rx closed :(");
                }
            }
        }

    }
}

pub struct Camera {
    pub mjpeg: watch::Receiver<Vec<u8>>,
}

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

fn spawn_camera(frame_tx: watch::Sender<Vec<u8>>, enable_rx: broadcast::Receiver<bool>) -> MjpegStreamState {
    let cancel_token = CancellationToken::new();
    let kill_switch = cancel_token.clone().cancelled_owned();
    let (callback_tx, callback_rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Ok(mut frames) = start_mjpeg_stream(kill_switch) {
            while let Ok(_) = frames.changed().await {
                let last_frame = frames.borrow();
                if let Some(lf) = last_frame.deref() {
                    frame_tx.send(lf.bytes.clone()).expect("Error sending camera frame");
                }
            }
            info!("camera mjpeg stream shut down!");
        } else {
            warn!("failed to start mjpeg stream!");
        }

        info!("sending tp!");
        let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
        frame_tx.send(tp.clone()).unwrap();
        tokio::spawn(async move {
           tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
           info!("sending tp!");
           frame_tx.send(tp).unwrap();
           callback_tx.send(frame_tx)
        });


    });

    let mut enable = enable_rx.resubscribe();
    tokio::spawn( async move {
        while let Ok(is_enabled) = enable.recv().await {
            if !is_enabled {
                break;
            }
        }
        cancel_token.cancel();
    });

    MjpegStreamState{ callback: callback_rx, enable_rx: enable_rx.resubscribe() }
}

impl Camera {

    pub fn new(enable: &dyn Input<bool>) -> Self {

        let (frame_tx, frame_rx) = watch::channel(TestPatternGenerator::new(640, 480, 50).generate().bytes);
        let enable_source = enable.source();

        let enable_rx = enable_source.rx;
        let mut state = if enable_source.start {
            CameraState::MjpegStream(spawn_camera(frame_tx, enable_rx))
        } else {
            CameraState::StreamEnded(StreamEndedState{ frame_tx, enable_rx })
        };
        
        tokio::spawn(async move {
            loop {
                state = state.step().await
            }
        });

        Self {
            mjpeg: frame_rx,
        }
    }
}