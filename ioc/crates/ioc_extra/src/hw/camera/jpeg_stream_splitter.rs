use crate::hw::camera::image::JpegImage;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{broadcast, watch};
use tracing::info;

enum SplitJpegState {
    Marker1,      //looking for 0xFF
    Marker2,      //looking for a second marker byte
    Size1,        //looking for first byte of payload size
    Size2(u8),    //looking for second byte of payload size
    Payload(u16), //iterating through a payload following a marker for n remaining bytes
    EntropyCoded, //iterating through entropy-coded data for 0xFF
    EcEscape,     //checking of next byte of 0x00 to escape entropy coded data
}

pub fn split_jpegs(
    mut byte_stream: impl AsyncRead + AsyncReadExt + Send + Unpin + 'static,
) -> broadcast::Receiver<Option<JpegImage>> {
    let (tx, rx) = broadcast::channel(1);

    let buffer_size: usize = 1024;
    let mut buf: Vec<u8> = vec![0; buffer_size];

    let frame_capacity: usize = 10000;
    let mut frame: Vec<u8> = Vec::with_capacity(frame_capacity);

    let mut state = SplitJpegState::Marker1;

    tokio::spawn(async move {
        while let Ok(bytes) = byte_stream.read(&mut buf).await {
            if bytes > 0 {
                let mut i = 0;

                while i < bytes {
                    let b = buf[i];
                    match state {
                        SplitJpegState::Marker1 => {
                            if b == 0xFF {
                                frame.push(b);
                                state = SplitJpegState::Marker2;
                            }
                            i += 1;
                        }

                        SplitJpegState::Marker2 => {
                            if b == 0xD8 {
                                //start of image!
                                state = SplitJpegState::Marker1;
                                frame.push(b);
                            } else if b == 0xD9 {
                                //end of image!
                                state = SplitJpegState::Marker1;
                                frame.push(b);

                                tx.send(Some(JpegImage {
                                    bytes: frame.clone(),
                                }))
                                .unwrap();
                                frame.clear();
                            } else if (0xD0..0xD7).contains(&b) {
                                //reset marker - no size
                                state = SplitJpegState::Marker1;
                                frame.push(b);
                            } else if b == 0xDA {
                                //begin entropy coded data
                                state = SplitJpegState::EntropyCoded;
                                frame.push(b);
                            } else {
                                //some other sized block of data
                                state = SplitJpegState::Size1;
                                frame.push(b);
                            }
                            i += 1;
                        }

                        SplitJpegState::Size1 => {
                            state = SplitJpegState::Size2(b);
                            frame.push(b);
                            i += 1;
                        }

                        SplitJpegState::Size2(first_byte) => {
                            //remaining bytes minus two for payload size
                            state = SplitJpegState::Payload(
                                ((first_byte as u16) << 8 | (b as u16)) - 2,
                            );
                            frame.push(b);
                            i += 1;
                        }

                        SplitJpegState::Payload(remaining_bytes) => {
                            if remaining_bytes == 0 {
                                state = SplitJpegState::Marker1;
                            } else {
                                state = SplitJpegState::Payload(remaining_bytes - 1);
                                i += 1;
                                frame.push(b);
                            }
                        }

                        SplitJpegState::EntropyCoded => {
                            if b == 0xFF {
                                state = SplitJpegState::EcEscape;
                            }
                            frame.push(b);
                            i += 1;
                        }

                        SplitJpegState::EcEscape => {
                            if b == 0x00 {
                                state = SplitJpegState::EntropyCoded;
                                frame.push(b);
                                i += 1;
                            } else {
                                state = SplitJpegState::Marker2;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }
        info!("child process stream ended.")
    });

    rx
}
