use axum::{
    routing::get,
    Router,
    response::Response,
    body::Body
};
use futures_util::Stream;
use tokio::sync::watch;
use std::pin::Pin;
use std::task::{Context, Poll};


pub struct MjpegStreamEndpoint {
    pub frames: watch::Receiver<Vec<u8>>,
}

#[derive(Clone)]
pub struct MjpegStream {
    rx: watch::Receiver<Vec<u8>>,
}

impl MjpegStream {
    pub fn new(rx: watch::Receiver<Vec<u8>>) -> Self {
        MjpegStream { rx }
    }
}

#[derive(Debug)]
pub struct MjpegStreamError {
    pub message: String,
}

impl std::fmt::Display for MjpegStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MjpegStreamError {

}

impl Stream for MjpegStream {
    type Item = Result<Vec<u8>, MjpegStreamError>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {

        if let Ok(changed) = self.rx.has_changed() {
            if changed {
                let mut frame = self.rx.borrow().clone();

                let mut bound = format!("--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", BOUNDARY, frame.len()).into_bytes();
                let mut bytes = Vec::with_capacity(frame.len() + bound.len());
                bytes.append(&mut bound);
                bytes.append(&mut frame);
                Poll::Ready(Some(Ok(bytes)))
            } else {
                Poll::Pending
            }
        } else {
            println!("stream closed!");
            Poll::Ready(Some(Err(MjpegStreamError{ message: "stream closed".to_string()})))
        }
    }
}


static BOUNDARY: &str = "lolthisismyboundlol";

impl MjpegStreamEndpoint {
    pub fn apply(self, key: &str, router: Router) -> Router {
        let rx = self.frames.clone();
        router.route(key, get(|| async move {
            let stream = MjpegStream::new(rx);
            let body = Body::wrap_stream(stream);   
            Response::builder().header("Content-Type", "multipart/x-mixed-replace;boundary=\"lolthisismyboundlol\"").status(200).body(body).unwrap()
        }))
    }
}
