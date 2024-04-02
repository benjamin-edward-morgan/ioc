use embedded_graphics::{framebuffer::buffer_size, pixelcolor::Rgb888};
use tokio::{process::ChildStdout, sync::{mpsc, watch}, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use super::{child_process_stream::start_child_process, image::TestFrameGenerator, jpeg_stream_splitter::split_jpegs};


struct CameraMjpegStream {
    cancel_token: CancellationToken,
}

impl CameraMjpegStream {
    fn new(frames: watch::Sender<Vec<u8>>, camevt_tx: mpsc::Sender<CameraEvt>, params: CameraParams) -> Self {

        frames.send(TestFrameGenerator::<640, 480, {buffer_size::<Rgb888>(640, 480)}>::new().with_text("starting camera stream...").build_jpeg()).unwrap();

        let args = params.get_libcamera_params();
        let stream_handler = |child_out: ChildStdout, frames_tx: watch::Sender<Vec<u8>>| split_jpegs(child_out, frames_tx);
        let cancel_token = CancellationToken::new();

        let join_handle = match start_child_process("libcamera-vid", &args, frames, stream_handler, cancel_token.clone()) {
            Ok(handle) => handle,
            Err(err) => {
                error!("error starting child process: {:?}", err.message);
                let frames = err.x;
                frames.send(TestFrameGenerator::<640, 480, {buffer_size::<Rgb888>(640, 480)}>::new().with_text("camera error!").build_jpeg()).unwrap();
                tokio::spawn(async move {
                    frames
                })
            },
        };

        tokio::spawn(async move {
            let frame_tx = join_handle.await.unwrap();
            camevt_tx.send(CameraEvt::StreamFinished(frame_tx)).await.unwrap();
        });

        Self{ cancel_token }
    }

    async fn transition(&mut self, params: &CameraParams) {
        debug!("camera mjpeg stream transition");
        if !params.enabled {
            debug!("cancelling stream!");
            self.cancel_token.cancel();
        } else {
            debug!("stream already running noop!");
        }
    }
}



struct CameraDisabled {
    frames: Option<watch::Sender<Vec<u8>>>,
    camevt_tx: mpsc::Sender<CameraEvt>,
}

impl CameraDisabled {
    fn new(frames: watch::Sender<Vec<u8>>, camevt_tx: mpsc::Sender<CameraEvt>) -> Self {
        frames.send(TestFrameGenerator::<640, 480, {buffer_size::<Rgb888>(640, 480)}>::new().with_text("camera disabled").build_jpeg()).unwrap();
        Self{ frames: Some(frames), camevt_tx }
    }

    async fn transition(&mut self, params: &CameraParams) {
        debug!("camera disabled transition {:?} has frames: {}", params, self.frames.is_some());
        if params.enabled {
            match self.frames.take() {
                Some(frames) => {
                    self.camevt_tx.send(CameraEvt::StreamFinished(frames)).await.unwrap();
                },
                None => {
                    debug!("no frames to send!");
                }
            }
        } else {
            debug!("disabled noop!");
        }
    }
}

enum CameraState {
    Disabled(CameraDisabled),
    MjpegStream(CameraMjpegStream),
}

impl CameraState {
    async fn transition(&mut self, params: &CameraParams) {
        match self {
            CameraState::Disabled(disabled) => disabled.transition(params).await,
            CameraState::MjpegStream(mjpeg) => mjpeg.transition(params).await,
        }
    }

}


#[derive(Debug, Clone)]
struct CameraParams {
    enabled: bool,
}

impl CameraParams {
    fn get_libcamera_params(&self) -> Vec<&str> {
        let params: Vec<&str> = vec![
            "--rotation", "180",
            "--width", "640",
            "--height", "480",
            "--codec", "mjpeg",
            "--framerate", "10",
            "--tuning-file", "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
            "--mode", "3280:2464:10:U", //mode makes sure to use the whole sensor, not cropping middle
            "-q", "25", //jpeg quality 
            "-t", "0", //no timeout - stream forever 
            "-n", //no preview window 
            "--flush", //flush output after each frame
            "-o", "-", //output to stdout
        ]; 
        params
    }
    
}

impl Default for CameraParams {
    fn default() -> Self {
        CameraParams {
            enabled: false,
        }
    }
}

fn spawn_watch_camera_params(
    params: &CameraParams,
    mut enable: mpsc::Receiver<bool>,
    params_tx: mpsc::Sender<CameraEvt>,
) -> JoinHandle<()> {
    let mut params = params.clone();
    tokio::spawn(async move {
        while let Some(enabled) = enable.recv().await {
            params.enabled = enabled;
            if let Err(_) = params_tx.send(CameraEvt::ParamsUpdated(params.clone())).await {
                break;
            }
        }
        debug!("camera params task shutting down!");
    })
}



enum CameraEvt {
    ParamsUpdated(CameraParams),
    StreamFinished(watch::Sender<Vec<u8>>),
}

pub(super) struct CameraManager {}


impl CameraManager {

    pub(super) fn spawn_camera_manager_task(
        enable: mpsc::Receiver<bool>,
        frames: watch::Sender<Vec<u8>>,
        cancel_token: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move{

            let (camevt_tx, mut camevt_rx) = mpsc::channel::<CameraEvt>(10);

            let mut params = CameraParams::default();
            let params_task = spawn_watch_camera_params(&params, enable, camevt_tx.clone());

            let mut state = CameraState::Disabled(CameraDisabled::new(frames, camevt_tx.clone()));

            loop {

                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        break;
                    },
                    cam_evt = camevt_rx.recv() => {
                        match cam_evt {
                            Some(CameraEvt::ParamsUpdated(new_params)) => {
                                debug!("new params! {:?}", new_params);
                                params = new_params.clone();
                                state.transition(&new_params).await;
                            },
                            Some(CameraEvt::StreamFinished(frames)) => {
                                debug!("stream finished!");
                                if !params.enabled {
                                    debug!("making stream disabled state");
                                    state = CameraState::Disabled(CameraDisabled::new(frames, camevt_tx.clone()));
                                } else {
                                    debug!("making stream mjpeg state");
                                    state = CameraState::MjpegStream(CameraMjpegStream::new(frames, camevt_tx.clone(), params.clone()));
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    },
                }
            }

            params_task.abort();
            debug!("camera manager task shutting down!");
        })
    }
}

