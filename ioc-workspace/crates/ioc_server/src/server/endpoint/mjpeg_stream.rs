use std::collections::HashSet;

use axum::{
    routing::get,
    Router,
    response::Response,
    body::Body
};
use futures_util::{Stream, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_stream::wrappers::WatchStream;
use tracing::warn;

use crate::server::state::{ServerOutputState, StateCmd};

pub struct MjpegStreamEndpoint {
    frames: watch::Receiver<Vec<u8>>,
}

impl MjpegStreamEndpoint {
    pub fn new(cmd_tx: &mpsc::Sender<StateCmd>, output: &str) -> Self {

        let (frames_tx, frames) = watch::channel(Vec::<u8>::new());

        let cmd_tx = cmd_tx.clone();
        let output = output.to_owned();
        tokio::spawn(async move {
            let (callback_tx, callback_rx) = oneshot::channel();
            let subs_cmd = StateCmd::Subscribe { 
                callback: callback_tx, 
                inputs: HashSet::new(), 
                outputs: HashSet::from([output.to_owned()]),
            };
            cmd_tx.send(subs_cmd).await.expect("unable to subscribe to video feed");
            let mut subs = callback_rx.await.expect("did not get video state subscription");

            while let Ok(update) = subs.update_rx.recv().await {
                if let Some(ServerOutputState::Binary { value: Some(frame) }) = update.outputs.get(&output) {
                    frames_tx.send(frame.clone()).expect("could not send frame to watch")
                }
            }
            warn!("mjpeg stream task shutting down!");
        });
        

        MjpegStreamEndpoint { frames }
    }

}

static BOUNDARY: &str = "lolthisismyboundlol";

fn as_mjpeg_stream(frames: watch::Receiver<Vec<u8>>) -> impl Stream<Item=Result<Vec<u8>,String>> {
        
    WatchStream::new(frames).map(|mut frame| {

        let mut bound = format!("--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", BOUNDARY, frame.len()).into_bytes();
        let mut bytes = Vec::with_capacity(frame.len() + bound.len());

        bytes.append(&mut bound);
        bytes.append(&mut frame);
        Ok(bytes)
    })
}

impl MjpegStreamEndpoint {
    pub fn apply(self, key: &str, router: Router) -> Router {
        let rx = self.frames.clone();
        router.route(key, get(|| async move {
            let body = Body::wrap_stream(as_mjpeg_stream(rx));   
            Response::builder().header("Content-Type", format!("multipart/x-mixed-replace;boundary=\"{}\"", BOUNDARY)).status(200).body(body).unwrap()
        }))
    }
}
