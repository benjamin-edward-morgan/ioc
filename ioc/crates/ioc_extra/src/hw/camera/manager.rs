use tokio::{process::ChildStdout, sync::{mpsc, watch}, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use super::{child_process_stream::start_child_process, image::TestFrameGenerator, jpeg_stream_splitter::split_jpegs};


struct CameraMjpegStream {
    cancel_token: CancellationToken,
}

impl CameraMjpegStream {
    fn new(frames: watch::Sender<Vec<u8>>, camevt_tx: mpsc::Sender<CameraEvt>, params: CameraParams) -> Self {

        frames.send(TestFrameGenerator::new(params.w, params.h).with_q(params.q).with_text("starting camera stream...").build_jpeg()).unwrap();

        let args = params.get_libcamera_params();
        let stream_handler = |child_out: ChildStdout, frames_tx: watch::Sender<Vec<u8>>| split_jpegs(child_out, frames_tx);
        let cancel_token = CancellationToken::new();

        let join_handle = match start_child_process("libcamera-vid", &args, frames, stream_handler, cancel_token.clone()) {
            Ok(handle) => handle,
            Err(err) => {
                error!("error starting child process: {:?}", err.message);
                let frames = err.x;
                frames.send(TestFrameGenerator::new(params.w, params.h).with_q(params.q).with_text("camera error!").build_jpeg()).unwrap();
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

    async fn transition(&mut self, _params: &CameraParams) {
        debug!("camera mjpeg stream transition");
        self.cancel_token.cancel();
    }
}



struct CameraDisabled {
    frames: Option<watch::Sender<Vec<u8>>>,
    camevt_tx: mpsc::Sender<CameraEvt>,
}

impl CameraDisabled {
    fn new(frames: watch::Sender<Vec<u8>>, camevt_tx: mpsc::Sender<CameraEvt>, params: CameraParams) -> Self {
        frames.send(TestFrameGenerator::new(params.w, params.h).with_q(params.q).with_text("camera disabled").build_jpeg()).unwrap();
        Self{ frames: Some(frames), camevt_tx }
    }

    async fn transition(&mut self, params: &CameraParams) {
        debug!("camera disabled transition {:?} has frames: {}", params, self.frames.is_some());
        match self.frames.take() {
            Some(frames) => {
                self.camevt_tx.send(CameraEvt::StreamFinished(frames)).await.unwrap();
            },
            None => {}
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
    q: u8, //from 0 to 100 inclusive
    framerate: u8, //from 1 to 60 inclusive
    w: usize, //resolution width 
    h: usize, //resolution height
}

impl CameraParams {
    fn get_libcamera_params(&self) -> Vec<String> {

        let mut params: Vec<String> = Vec::with_capacity(50);

        for arg in [
            "--rotation", "180",
            "--codec", "mjpeg",
            "--tuning-file", "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
            "--mode", "3280:2464:10:U", //mode makes sure to use the whole sensor, not cropping middle
        ] {
            params.push(arg.to_string());
        }

        params.push("-q".to_string());
        params.push(self.q.to_string());

        params.push("--framerate".to_string());
        params.push(self.framerate.to_string());

        params.push("--width".to_string());
        params.push(self.w.to_string());

        params.push("--height".to_string());
        params.push(self.h.to_string());

        for arg in [
                "-t", "0", //no timeout - stream forever 
                "-n", //no preview window 
                "--flush", //flush output after each frame
                "-o", "-" //output to std out
        ] {
            params.push(arg.to_string());
        }
      
        params
    }
}

impl Default for CameraParams {
    fn default() -> Self {
        CameraParams {
            enabled: false,
            q: 50,
            framerate: 5,
            w: 640,
            h: 480,
        }
    }
}

fn spawn_watch_camera_params(
    params: &CameraParams,
    mut enable: mpsc::Receiver<bool>,
    mut q: mpsc::Receiver<f64>,
    mut framerate: mpsc::Receiver<f64>,
    mut resolution: mpsc::Receiver<String>,
    params_tx: mpsc::Sender<CameraEvt>,
) -> JoinHandle<()> {
    let mut params = params.clone();
    tokio::spawn(async move {
        loop {
            tokio::select!{
                enable_o = enable.recv() => {
                    if let Some(enabled) = enable_o {
                        params.enabled = enabled;
                        if let Err(_) = params_tx.send(CameraEvt::ParamsUpdated(params.clone())).await {
                            break;
                        }
                    } else {
                        break;
                    }
                },
                q_o = q.recv() => {
                    if let Some(q_val) = q_o {
                        params.q = q_val.max(0.0).min(100.0) as u8;
                        if let Err(_) = params_tx.send(CameraEvt::ParamsUpdated(params.clone())).await {
                            break;
                        }
                    } else {
                        break;
                    }
                },
                framerate_o = framerate.recv() => {
                    if let Some(framerate_val) = framerate_o {
                        params.framerate = framerate_val.max(1.0).min(60.0) as u8;
                        if let Err(_) = params_tx.send(CameraEvt::ParamsUpdated(params.clone())).await {
                            break;
                        }
                    } else {
                        break;
                    }
                },
                resolution_o = resolution.recv() => {
                    if let Some(resolution_val) = resolution_o {
                        let parts: Vec<&str> = resolution_val.split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(w), Ok(h)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                params.w = w;
                                params.h = h;
                                if let Err(_) = params_tx.send(CameraEvt::ParamsUpdated(params.clone())).await {
                                    break;
                                }
                            } else {
                                error!("invalid resolution string: {}", resolution_val);
                            }
                        } else {
                            error!("invalid resolution string: {}", resolution_val);
                        }
                    } else {
                        break;
                    }
                }
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
        q: mpsc::Receiver<f64>,
        framerate: mpsc::Receiver<f64>,
        resolution: mpsc::Receiver<String>,
        frames: watch::Sender<Vec<u8>>,
        cancel_token: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move{

            let (camevt_tx, mut camevt_rx) = mpsc::channel::<CameraEvt>(10);

            let mut params = CameraParams::default();
            let params_task = spawn_watch_camera_params(&params, enable, q, framerate, resolution, camevt_tx.clone());

            let mut state = CameraState::Disabled(CameraDisabled::new(frames, camevt_tx.clone(), params.clone()));

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
                                    state = CameraState::Disabled(CameraDisabled::new(frames, camevt_tx.clone(), params.clone()));
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

