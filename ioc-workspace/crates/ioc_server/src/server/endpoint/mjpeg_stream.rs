use axum::{
    routing::get,
    Router,
    response::Response,
    body::Body
};
use futures_util::{Stream, StreamExt};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

pub struct MjpegStreamEndpoint {
    pub frames: watch::Receiver<Vec<u8>>,
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
