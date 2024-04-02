
use jpeg_encoder::Encoder;


use embedded_graphics::{
    framebuffer::Framebuffer, 
    mono_font::{ascii::FONT_6X10, MonoTextStyle}, 
    pixelcolor::{raw::LittleEndian, Rgb888}, 
    prelude::*, 
    text::Text
};


pub struct TestFrameGenerator<const W: usize, const H: usize, const BUF: usize> {
    q: u8,
    text: Option<String>
}

impl <const W: usize, const H: usize, const BUF: usize> TestFrameGenerator<W, H, BUF> {
    pub fn new() -> Self {
        Self { q: 50, text: None }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_owned());
        self
    }

    pub fn with_q(&mut self, q: u8) -> &mut Self {
        self.q = q;
        self
    }

    pub fn build_jpeg(self) -> Vec<u8> {
        let mut jpeg_buffer = Vec::with_capacity(2048);
        let encoder: Encoder<&mut Vec<u8>> = Encoder::new(&mut jpeg_buffer, self.q);
        // const BUF_SIZE: usize = buffer_size::<Rgb888>(W, H);
        let mut framebuffer = Framebuffer::<Rgb888, _, LittleEndian, W, H, BUF>::new();
      
        //add text to frame
        if let Some(txt) = self.text {
            let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
            let txt = Text::new(&txt, Point::new(20, 20), style);
            txt.draw(&mut framebuffer).unwrap();
        }

        //encode as jpeg and return buffer
        encoder.encode(framebuffer.data(), W as u16, H as u16, jpeg_encoder::ColorType::Rgb).unwrap();
        jpeg_buffer
    }
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

    pub fn generate(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(1024);

        let encoder: Encoder<&mut Vec<u8>> = Encoder::new(&mut buffer, self.q);

        let mut raw_rgb = vec![0u8; (self.w as usize) * (self.h as usize) * 3];

        // for x in 0..self.w {
        //     for y in 0..self.h {
        //         let rgb: u32;

        //         if y < self.h * 3 / 4 {
        //             if x < self.w / 7 {
        //                 rgb = 0x00FFFFFF;
        //             } else if x < self.w * 2 / 7 {
        //                 rgb = 0x00FFFE54;
        //             } else if x < self.w * 3 / 7 {
        //                 rgb = 0x0054FFFF;
        //             } else if x < self.w * 4 / 7 {
        //                 rgb = 0x0054FF54;
        //             } else if x < self.w * 5 / 7 {
        //                 rgb = 0x00FF54FF;
        //             } else if x < self.w * 6 / 7 {
        //                 rgb = 0x00FF5454;
        //             } else {
        //                 rgb = 0x005454FF;
        //             }
        //         } else {
        //             let q = (x as f32) / (self.w as f32);
        //             let q = (q * 255.0) as u8;
        //             rgb = (q as u32) << 16 | (q as u32) << 8 | q as u32;
        //         }

        //         let idx = ((y as usize) * (self.w as usize) + x as usize) * 3;
        //         raw_rgb[idx] = (rgb & 0xFF) as u8;
        //         raw_rgb[idx + 1] = ((rgb >> 8) & 0xFF) as u8;
        //         raw_rgb[idx + 2] = ((rgb >> 16) & 0xFF) as u8;
        //     }
        // }

        encoder
            .encode(&raw_rgb, self.w, self.h, jpeg_encoder::ColorType::Rgb)
            .unwrap();

        buffer 
    }
}
