mod image;
mod child_process_stream;
mod jpeg_stream_splitter;

use std::{collections::HashMap, ops::Deref};
use futures::Future;
use ioc_core::{error::IocBuildError, Input, InputKind, Module, ModuleBuilder, ModuleIO, OutputKind};
use serde::Deserialize;
use tokio::{sync::{broadcast, mpsc, oneshot, watch}, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use image::TestPatternGenerator;
use tracing::{info, warn};
use child_process_stream::start_child_process;
use jpeg_stream_splitter::split_jpegs;
use crate::input::{SimpleInput, SimpleOutput};

use self::{child_process_stream::ChildProcessError, image::JpegImage};

// struct MjpegStreamState {
//     pub callback: oneshot::Receiver<broadcast::Sender<Vec<u8>>>,
//     pub enable_rx: mpsc::Receiver<bool>,
// }

// struct StreamEndedState {
//     pub frame_tx: broadcast::Sender<Vec<u8>>,
//     pub enable_rx: mpsc::Receiver<bool>,
// }

// enum CameraState {
//     MjpegStream(MjpegStreamState),
//     StreamEnded(StreamEndedState),
// }

// impl CameraState {
//     async fn step(self) -> Self {
//         match self {
//             Self::MjpegStream(stream) => {
//                 if let Ok(frame_tx) = stream.callback.await {
//                     let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
//                     frame_tx.send(tp).unwrap();
//                     Self::StreamEnded(StreamEndedState{frame_tx, enable_rx: stream.enable_rx})
//                 } else {
//                     panic!("did not get frame_tx from callback :(");
//                 }
//             },
//             Self::StreamEnded(mut ended) => {
//                 let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
//                 ended.frame_tx.send(tp.clone()).unwrap();
//                 if let Some(is_enabled) = ended.enable_rx.recv().await {
//                     if is_enabled {
//                         Self::MjpegStream(spawn_camera(ended.frame_tx, ended.enable_rx))
//                     } else {
//                         Self::StreamEnded(ended)
//                     }
//                 } else {
//                     panic!("enable_rx closed :(");
//                 }
//             }
//         }

//     }
// }



fn start_mjpeg_stream(/*kill_switch: impl Future<Output = ()> + Send + 'static*/) -> Result<broadcast::Receiver<Option<JpegImage>>, ChildProcessError> {
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
        // kill_switch,
    )
}

// fn spawn_camera(frame_tx: broadcast::Sender<Vec<u8>>, mut enable_rx: mpsc::Receiver<bool>) -> MjpegStreamState {
//     let cancel_token = CancellationToken::new();
//     let kill_switch = cancel_token.clone().cancelled_owned();
//     let (callback_tx, callback_rx) = oneshot::channel();
//     tokio::spawn(async move {
//         if let Ok(mut frames) = start_mjpeg_stream(kill_switch) {
//             while frames.changed().await.is_ok() {
//                 let last_frame = frames.borrow();
//                 if let Some(lf) = last_frame.deref() {
//                     frame_tx.send(lf.bytes.clone()).expect("Error sending camera frame");
//                 }
//             }
//             info!("camera mjpeg stream shut down!");
//         } else {
//             warn!("failed to start mjpeg stream!");
//         }

//         info!("sending tp!");
//         let tp: Vec<u8> = TestPatternGenerator::new(640, 480, 50).generate().bytes;
//         frame_tx.send(tp.clone()).unwrap();
//         tokio::spawn(async move {
//            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//            info!("sending tp!");
//            frame_tx.send(tp).unwrap();
//            callback_tx.send(frame_tx)
//         });


//     });

//     tokio::spawn( async move {
//         while let Some(is_enabled) = enable_rx.recv().await {
//             if !is_enabled {
//                 break;
//             }
//         }
//         cancel_token.cancel();
//     });

//     MjpegStreamState{ callback: callback_rx }
// }

pub struct Camera {
    pub join_handle: JoinHandle<()>,
    pub mjpeg: SimpleInput<Vec<u8>>,
    pub enable: SimpleOutput<bool>,
}

impl From<Camera> for ModuleIO {
    fn from(cam: Camera) -> Self {
        ModuleIO { 
            join_handle: cam.join_handle, 
            outputs: HashMap::from([
                ("enabled".to_owned(), OutputKind::bool(cam.enable))
            ]),
            inputs: HashMap::from([
                ("mjpeg".to_owned(), InputKind::binary(cam.mjpeg))
            ])
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CameraConfig {

}


impl Module for Camera {
    type Config = CameraConfig;

    async fn try_build(_cfg: &CameraConfig) -> Result<Self, IocBuildError>  {
        let (enable_tx, enable_rx) = mpsc::channel(10);
        let enable = SimpleOutput{ tx: enable_tx };

        let mut jpeg_rx = start_mjpeg_stream().unwrap();

        let (frame_tx, frame_rx) = broadcast::channel(1);

        let mjpeg = SimpleInput::new(Vec::new(), frame_rx);

        let join_handle = tokio::spawn(async move {
            while let Ok(jpeg) = jpeg_rx.recv().await {
                if let Some(jpeg) = jpeg {
                    frame_tx.send(jpeg.bytes).unwrap();
                }
            }
        });

        Ok(
            Self { join_handle, mjpeg, enable }
        )
    }
}
