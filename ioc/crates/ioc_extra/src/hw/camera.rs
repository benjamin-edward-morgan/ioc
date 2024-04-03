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


pub struct Camera {
    pub join_handle: JoinHandle<()>,
    pub mjpeg: Input<Vec<u8>>,
    pub enable: Output<bool>,
    pub q: Output<f64>,
    pub framerate: Output<f64>,
}

impl From<Camera> for ModuleIO {
    fn from(cam: Camera) -> Self {
        ModuleIO {
            join_handle: cam.join_handle,
            outputs: HashMap::from([
                ("enable".to_owned(), OutputKind::Bool(cam.enable)),
                ("quality".to_owned(), OutputKind::Float(cam.q)),
                ("framerate".to_owned(), OutputKind::Float(cam.framerate)),
            ]),
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
        let (q, q_rx) = Output::new();
        let (framerate, framerate_rx) = Output::new();

        let join_handle = CameraManager::spawn_camera_manager_task(enable_rx, q_rx, framerate_rx, frame_tx, cancel_token);

        Ok(Self {
            join_handle,
            mjpeg,
            enable,
            q,
            framerate,
        })
    }
}
