

use std::{borrow::BorrowMut, io::BufWriter};

use jpeg_encoder::Encoder;

pub struct JpegImage {
    pub bytes: Vec<u8>
}

pub struct TestPatternGenerator {
    w: u16, 
    h: u16, 
    q: u8,
}

impl TestPatternGenerator {

    pub fn new(w: u16, h: u16, q: u8) -> Self {
        Self { w, h, q }
    }

    pub fn generate(&self) -> JpegImage {

        let mut buffer = Vec::with_capacity(1024);

        let encoder = Encoder::new(&mut buffer, self.q);
        // let encoder = Encoder::new_file("foobar.jpeg", self.q).unwrap();

        let mut raw_rgb = vec![0u8 ; (self.w as usize)*(self.h as usize)*3];

        for x in 0..self.w {
            for y in 0..self.h {
                let idx = ((y as usize)*(self.w as usize) + x as usize) * 3;
                raw_rgb[idx] = ((y % 64) * 8) as u8;
                raw_rgb[idx+1] = ((x % 64) * 8) as u8;
            }
        }

        encoder.encode(&raw_rgb, self.w, self.h, jpeg_encoder::ColorType::Rgb).unwrap();

        JpegImage { bytes: buffer }
    }
}
