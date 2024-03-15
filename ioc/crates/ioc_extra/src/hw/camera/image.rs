use jpeg_encoder::Encoder;

#[derive(Clone, Debug)]
pub struct JpegImage {
    pub bytes: Vec<u8>,
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

        let mut raw_rgb = vec![0u8; (self.w as usize) * (self.h as usize) * 3];

        for x in 0..self.w {
            for y in 0..self.h {
                let rgb: u32;

                if y < self.h * 3 / 4 {
                    if x < self.w / 7 {
                        rgb = 0x00FFFFFF;
                    } else if x < self.w * 2 / 7 {
                        rgb = 0x00FFFE54;
                    } else if x < self.w * 3 / 7 {
                        rgb = 0x0054FFFF;
                    } else if x < self.w * 4 / 7 {
                        rgb = 0x0054FF54;
                    } else if x < self.w * 5 / 7 {
                        rgb = 0x00FF54FF;
                    } else if x < self.w * 6 / 7 {
                        rgb = 0x00FF5454;
                    } else {
                        rgb = 0x005454FF;
                    }
                } else {
                    let q = (x as f32) / (self.w as f32);
                    let q = (q * 255.0) as u8;
                    rgb = (q as u32) << 16 | (q as u32) << 8 | q as u32;
                }

                let idx = ((y as usize) * (self.w as usize) + x as usize) * 3;
                raw_rgb[idx] = (rgb & 0xFF) as u8;
                raw_rgb[idx + 1] = ((rgb >> 8) & 0xFF) as u8;
                raw_rgb[idx + 2] = ((rgb >> 16) & 0xFF) as u8;
            }
        }

        encoder
            .encode(&raw_rgb, self.w, self.h, jpeg_encoder::ColorType::Rgb)
            .unwrap();

        JpegImage { bytes: buffer }
    }
}
