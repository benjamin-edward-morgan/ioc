use axum::{
    routing::get,
    Router,
    response::Response,
    body::Body
};
use futures_util::Stream;
// use tokio::sync::broadcast;
// use tokio::sync::broadcast::error::TryRecvError;
use tokio::sync::watch;
use std::borrow::BorrowMut;
use std::pin::Pin;
use std::sync::Arc;
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {

        // match self.rx.try_recv() {
        //     Ok(mut frame) => {
        //         let mut bound = format!("--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", BOUNDARY, frame.len()).into_bytes();
        //         let mut bytes = Vec::with_capacity(frame.len() + bound.len());
        //         bytes.append(&mut bound);
        //         bytes.append(&mut frame);
        //         Poll::Ready(Some(Ok(bytes)))
        //     },
        //     Err(TryRecvError::Closed) => {
        //         println!("stream closed!");
        //         Poll::Ready(Some(Err(MjpegStreamError{ message: "stream closed".to_string()})))
        //     },
        //     Err(TryRecvError::Empty) => {
        //         Poll::Pending
        //     },
        //     Err(TryRecvError::Lagged(num_lagged)) => {
        //         println!("lagged! {}", num_lagged);
        //         Poll::Pending
        //     }
        // }
        if let Ok(changed) = self.rx.has_changed() {
            if(changed) {
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
        println!("applying!!!1");
        let rx = self.frames.clone();
        router.route(key, get(|| async move {

            println!("routing!!!");
            // let mut img1_dat = std::fs::read("assets/test1.jpeg").unwrap();
            // let mut img2_dat = std::fs::read("assets/test2.jpeg").unwrap();

            // let mut crlf = Vec::from("\r\n".as_bytes());

            // let bound_str1 = format!("--lolthisismyboundlol\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", img1_dat.len());
            // let bound_str2 = format!("--lolthisismyboundlol\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", img2_dat.len());


            // let mut img1 = Vec::from(bound_str1.as_bytes());
            // img1.append(&mut img1_dat);
            // img1.append(&mut crlf);

            // let mut img2 = Vec::from(bound_str2.as_bytes());
            // img2.append(&mut img2_dat);
            // img2.append(&mut crlf);

            // let mut i = 0;
            // let stream = futures_util::stream::repeat_with(move || {
            //     i += 1;
            //     sleep(Duration::from_millis(10));
            //     if i % 2 == 0 {
            //         Ok::<Vec<u8>,String>(img1.clone())
            //     } else {
            //         Ok::<Vec<u8>,String>(img2.clone())
            //     }
            // });

            let stream = MjpegStream::new(rx);

            println!("body!!1");
            let body = Body::wrap_stream(stream);
            
            // let body = Body::empty();

            println!("response!!");
            Response::builder().header("Content-Type", "multipart/x-mixed-replace;boundary=\"lolthisismyboundlol\"").status(200).body(body).unwrap()
        }))
    }

}
