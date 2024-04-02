mod child_process_stream;
mod image;
mod manager;
mod jpeg_stream_splitter;

use ioc_core::{
    error::IocBuildError, Input, InputKind, Module, ModuleIO, Output, OutputKind
};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;
use tokio::task::JoinHandle;
use manager::CameraManager;


// fn start_mjpeg_stream(cancel_token: CancellationToken) -> Result<broadcast::Receiver<Option<JpegImage>>, ChildProcessError> {
//     let args = [
//         "--rotation",
//         "180",
//         "--width",
//         "640",
//         "--height",
//         "480",
//         "--codec",
//         "mjpeg",
//         "--framerate",
//         "10",
//         "--tuning-file",
//         "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
//         "--mode", //mode makes sure to use the whole sensor, not cropping middle
//         "3280:2464:10:U",
//         "-q",
//         "25",
//         "-t",
//         "0",
//         "-n",
//         "--flush",
//         "-o",
//         "-",
//     ];
//     start_child_process(
//         "libcamera-vid",
//         &args,
//         split_jpegs,
//         cancel_token,
//     )
// }

pub struct Camera {
    pub join_handle: JoinHandle<()>,
    pub mjpeg: Input<Vec<u8>>,
    pub enable: Output<bool>,
}

impl From<Camera> for ModuleIO {
    fn from(cam: Camera) -> Self {
        ModuleIO {
            join_handle: cam.join_handle,
            outputs: HashMap::from([("enable".to_owned(), OutputKind::Bool(cam.enable))]),
            inputs: HashMap::from([("mjpeg".to_owned(), InputKind::Binary(cam.mjpeg))]),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct CameraConfig {}

impl Module for Camera {
    type Config = CameraConfig;

    async fn try_build(_cfg: &CameraConfig, cancel_token: CancellationToken) -> Result<Self, IocBuildError> {


        let (mjpeg, frame_tx) = Input::new(Vec::new());
        let (enable, enable_rx) = Output::new();

        let join_handle = CameraManager::spawn_camera_manager_task(enable_rx, frame_tx, cancel_token);

        Ok(Self {
            join_handle,
            mjpeg,
            enable,
        })
    }
}
