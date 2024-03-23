mod child_process_stream;
mod image;
mod jpeg_stream_splitter;

use child_process_stream::start_child_process;
use ioc_core::{
    error::IocBuildError, Input, InputKind, Module, ModuleIO
};
use jpeg_stream_splitter::split_jpegs;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::{
    sync::broadcast,
    task::JoinHandle,
};
use tracing::warn;

use self::{child_process_stream::ChildProcessError, image::JpegImage};


fn start_mjpeg_stream() -> Result<broadcast::Receiver<Option<JpegImage>>, ChildProcessError> {
    let args = [
        "--rotation",
        "180",
        "--width",
        "640",
        "--height",
        "480",
        "--codec",
        "mjpeg",
        "--framerate",
        "5",
        "--tuning-file",
        "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
        "--mode", //mode makes sure to use the whole sensor, not cropping middle
        "3280:2464:10:U",
        "-q",
        "50",
        "-t",
        "0",
        "-n",
        "--flush",
        "-o",
        "-",
    ];
    start_child_process(
        "libcamera-vid",
        &args,
        split_jpegs,
    )
}

pub struct Camera {
    pub join_handle: JoinHandle<()>,
    pub mjpeg: Input<Vec<u8>>,
}

impl From<Camera> for ModuleIO {
    fn from(cam: Camera) -> Self {
        ModuleIO {
            join_handle: cam.join_handle,
            outputs: HashMap::new(),
            inputs: HashMap::from([("mjpeg".to_owned(), InputKind::Binary(cam.mjpeg))]),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CameraConfig {}

impl Module for Camera {
    type Config = CameraConfig;

    async fn try_build(_cfg: &CameraConfig) -> Result<Self, IocBuildError> {


        let (mjpeg, frame_tx) = Input::new(Vec::new());

        let join_handle = tokio::spawn(async move {
            match start_mjpeg_stream() {
                Ok(mut jpeg_rx) => {
                    while let Ok(jpeg) = jpeg_rx.recv().await {
                        if let Some(jpeg) = jpeg {
                            frame_tx.send(jpeg.bytes).unwrap();
                        }
                    }
                }
                Err(err) => {
                    warn!("Child process failed: {:?}", err);
                }
            }
        });

        Ok(Self {
            join_handle,
            mjpeg,
        })
    }
}
