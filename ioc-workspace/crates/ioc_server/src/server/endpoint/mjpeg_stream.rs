use axum::{
    routing::get,
    Router
};


use axum::response::Response;
use axum::body::{Body};
use futures_util::StreamExt;
use std::thread::{Thread, sleep};
use std::time::Duration;
// use http::Response;
// use http_body::{Frame,Body};
use std::{pin::Pin, task::{Context, Poll}};
use bytes::{Buf, Bytes};


pub struct MjpegStreamEndpoint {

}


impl MjpegStreamEndpoint {

    pub fn apply(self, key: &str, router: Router) -> Router {



        router.route(key, get(|| async move {


            let mut img1_dat = std::fs::read("assets/test1.jpeg").unwrap();
            let mut img2_dat = std::fs::read("assets/test2.jpeg").unwrap();

            let mut crlf = Vec::from("\r\n".as_bytes());

            let bound_str1 = format!("--lolthisismyboundlol\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", img1_dat.len());
            let bound_str2 = format!("--lolthisismyboundlol\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", img2_dat.len());


            let mut img1 = Vec::from(bound_str1.as_bytes());
            img1.append(&mut img1_dat);
            img1.append(&mut crlf);

            let mut img2 = Vec::from(bound_str2.as_bytes());
            img2.append(&mut img2_dat);
            img2.append(&mut crlf);


            let mut i = 0;
            let stream = futures_util::stream::repeat_with(move || {
                i += 1;
                sleep(Duration::from_millis(100));
                if i % 2 == 0 {
                    Ok::<Vec<u8>,String>(img1.clone())
                } else {
                    Ok::<Vec<u8>,String>(img2.clone())
                }
            });

            Response::builder().header("Content-Type", "multipart/x-mixed-replace;boundary=\"lolthisismyboundlol\"").status(200).body(Body::wrap_stream(stream)).unwrap()
        }))
    }

}

// pub fn handle() -> Response<MjpegBody> {
//     Response::builder().status(200).body(MjpegBody{}).unwrap()
// }


// pub struct MjpegBody {

// }

// impl Body for MjpegBody {
//     type Data = Bytes;
//     type Error = String;

//     fn poll_frame(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
//         todo!();
//     }
// }
