
use tokio::sync::watch;
use std::process::{Command, Stdio, ChildStdout};
use std::io::Read;

pub struct ChildProcessInput {
    pub rx: watch::Receiver<Vec<u8>>,
}

impl Default for ChildProcessInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildProcessInput {
    pub fn new() -> Self {

        let (tx, rx) = watch::channel(Vec::new());

        println!("spawing child waiter!");
        std::thread::spawn(move || {

            let mut child = Command::new("libcamera-vid")
                .args([
                    "--rotation", "180",
                    "--width", "320",
                    "--height","240",
                    "--codec", "mjpeg",
                    "--framerate", "5",
                    "--tuning-file", "/usr/share/libcamera/ipa/rpi/vc4/imx219_noir.json",
                    "-q", "50",
                    "-t", "0",
                    "-n",
                    "--flush",
                    "-o", "-"
                ])
                .stderr(Stdio::inherit())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn child process!");

            println!("making jpeg chopper!");    
            let child_out = child.stdout.take().unwrap();
            chop_jpegs(child_out, tx);

            println!("done chopping jpegs!");

            child.wait().expect("child wait failed!");
            println!("child done!");
        });

        println!("returning child proc!");
        Self {
            rx,
        }

    }
}


#[derive(Debug)]
enum JpegChopperState {
    Marker1, //looking for 0xFF
    Marker2, //looking for a second marker byte
    Size1, //looking for first byte of payload size
    Size2(u8), //looking for second byte of payload size
    Payload(u16), //iterating through a payload following a marker for n remaining bytes
    EntropyCoded, //iterating through entropy-coded data for 0xFF
    EcEscape, //checking of next byte of 0x00 to escape entropy coded data
}


// pub struct JpegChopper ;

// impl JpegChopper {
//     pub fn new(mut stream: ChildStdout, tx: watch::Sender<Vec<u8>>) -> JpegChopper  {

        // tokio::spawn(async move {

fn chop_jpegs(mut stream: ChildStdout, tx: watch::Sender<Vec<u8>>) {
            let mut frame: Vec<u8> = Vec::with_capacity(1000000);
            let mut state = JpegChopperState::Marker1;
            let mut buf: Vec<u8> = vec![0; 1024];

            while let Ok(bytes) = stream.read(buf.as_mut_slice()) {
                if bytes > 0 {
                    let mut i = 0;

                    while i < bytes {
                        let b = buf[i];
                        match state {
                            JpegChopperState::Marker1 => {
                                if b == 0xFF {
                                    frame.push(b);
                                    state = JpegChopperState::Marker2;
                                }
                                i += 1;
                            },

                            JpegChopperState::Marker2 => {
                                if b == 0xD8 {
                                    //start of image! 
                                    state = JpegChopperState::Marker1;
                                    frame.push(b);
                                } else if b == 0xD9 {
                                    //end of image! 
                                    state = JpegChopperState::Marker1;
                                    frame.push(b);

                                    tx.send(frame.clone()).unwrap();
                                    frame.clear();
                                } else if (0xD0..0xD7).contains(&b) {
                                    //reset marker - no size
                                    state = JpegChopperState::Marker1;
                                    frame.push(b);
                                } else if b == 0xDA {
                                    //begin entropy coded data
                                    state = JpegChopperState::EntropyCoded;
                                    frame.push(b);
                                } else {
                                    //some other sized block of data
                                    state = JpegChopperState::Size1;
                                    frame.push(b);
                                }
                                i += 1;
                            }

                            JpegChopperState::Size1 => {
                                state = JpegChopperState::Size2(b);
                                frame.push(b);
                                i += 1;
                            }

                            JpegChopperState::Size2(first_byte) => {
                                //remaining bytes minus two for payload size
                                state = JpegChopperState::Payload(((first_byte as u16) << 8 | (b as u16)) - 2);
                                frame.push(b);
                                i += 1;
                            }

                            JpegChopperState::Payload(remaining_bytes) => {

                                if remaining_bytes == 0 {
                                    state = JpegChopperState::Marker1;
                                } else {
                                    state = JpegChopperState::Payload(remaining_bytes - 1);
                                    i += 1;
                                    frame.push(b);
                                }

                                // if (remaining_bytes as usize) < bytes - i - 1 {
                                //     frame.extend_from_slice(&buf[i..(i+remaining_bytes as usize)]);
                                //     i += remaining_bytes as usize;
                                //     state = JpegChopperState::Marker1;
                                // } else {
                                //     let len = bytes - i;
                                //     frame.extend_from_slice(&buf[i..(i+len)]);
                                //     i += len;
                                //     state = JpegChopperState::Payload(remaining_bytes - len as u16)
                                // }
                            }

                            JpegChopperState::EntropyCoded => {
                                if b == 0xFF {
                                    state = JpegChopperState::EcEscape;
                                }
                                frame.push(b);
                                i += 1;
                            }

                            JpegChopperState::EcEscape => {
                                if b == 0x00 {
                                    state = JpegChopperState::EntropyCoded;
                                    frame.push(b);
                                    i += 1;
                                } else {
                                    state = JpegChopperState::Marker2;
                                }
                                
                            },
                        }
                    }
                    
                } else {
                    break;
                }
            }
            println!("done with child process std output stream");
}