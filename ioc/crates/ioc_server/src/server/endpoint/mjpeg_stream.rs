use std::collections::HashSet;

use axum::{body::Body, response::Response, routing::get, Router};
use futures_util::{Stream, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_stream::wrappers::WatchStream;
use tracing::debug;

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
            cmd_tx
                .send(subs_cmd)
                .await
                .expect("unable to subscribe to video feed");
            let mut subs = callback_rx
                .await
                .expect("did not get video state subscription");

            while let Ok(update) = subs.update_rx.recv().await {
                if let Some(ServerOutputState::Binary { value: Some(frame) }) =
                    update.outputs.get(&output)
                {
                    frames_tx
                        .send(frame.clone())
                        .expect("could not send frame to watch")
                }
            }
            debug!("mjpeg stream task shutting down!");
        });

        MjpegStreamEndpoint { frames }
    }
}

static BOUNDARY: &str = "lolthisismyboundlol";

fn as_mjpeg_stream(
    frames: watch::Receiver<Vec<u8>>,
) -> impl Stream<Item = Result<Vec<u8>, String>> {
    let remaining = WatchStream::new(frames).map(|mut frame| {
        if !frame.is_empty() {
            debug!("emit nonempty frame! {}", frame.len());
            let mut bound0 = format!(
                "Content-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                frame.len()
            )
            .into_bytes();
            let mut bound1 = format!(
                "\r\n--{}\r\n",
                BOUNDARY,
            ).into_bytes();

            let mut bytes = Vec::with_capacity(frame.len() + bound0.len() + bound1.len());

            bytes.append(&mut bound0);
            bytes.append(&mut frame);
            bytes.append(&mut bound1);
            Ok(bytes)
        } else {
            debug!("emit empty frame!");
            Ok(vec![])
        }
    });

    let start = format!("--{}\r\n", BOUNDARY).into_bytes();
    let start = tokio_stream::once(Ok(start));

    start.chain(remaining)
}

impl MjpegStreamEndpoint {
    pub fn apply(self, key: &str, router: Router) -> Router {
        let rx = self.frames.clone();
        router.route(
            key,
            get(|| async move {
                let body = Body::wrap_stream(as_mjpeg_stream(rx));
                Response::builder()
                    .header(
                        "Content-Type",
                        format!("multipart/x-mixed-replace;boundary=\"{}\"", BOUNDARY),
                    )
                    .status(200)
                    .body(body)
                    .unwrap()
            }),
        )
    }
}
