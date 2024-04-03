
use jpeg_encoder::Encoder;


use embedded_graphics::{
    framebuffer::{buffer_size, Framebuffer}, 
    mono_font::{ascii::FONT_10X20, MonoTextStyle}, 
    pixelcolor::{raw::{LittleEndian, RawU24}, Rgb888}, 
    prelude::*, 
    primitives::Rectangle, 
    text::{renderer::CharacterStyle, Alignment, Text}
};

//these boxes are necessary to avoid sending large objects on the stack, causing stack overflow!
pub enum SimpleFrameBuffer {
    _320x240(Box<Framebuffer<Rgb888, RawU24, LittleEndian, 320, 240, {buffer_size::<Rgb888>(320, 240)}>>),
    _640x480(Box<Framebuffer<Rgb888, RawU24, LittleEndian, 640, 480, {buffer_size::<Rgb888>(640, 480)}>>),
    _1280x720(Box<Framebuffer<Rgb888, RawU24, LittleEndian, 1280, 720, {buffer_size::<Rgb888>(1280, 720)}>>),
    _1920x1080(Box<Framebuffer<Rgb888, RawU24, LittleEndian, 1920, 1080, {buffer_size::<Rgb888>(1920, 1080)}>>),
}

impl SimpleFrameBuffer {
    fn new_320x240() -> Self {
        Self::_320x240(Box::new(Framebuffer::new()))
    }
    fn new_640x480() -> Self {
        Self::_640x480(Box::new(Framebuffer::new()))
    }
    fn new_1280x720() -> Self {
        Self::_1280x720(Box::new(Framebuffer::new()))
    }
    fn new_1920x1080() -> Self {
        Self::_1920x1080(Box::new(Framebuffer::new()))
    }

    fn data(&self) -> &[u8] {
        match self {
            Self::_320x240(fb) => fb.data(),
            Self::_640x480(fb) => fb.data(),
            Self::_1280x720(fb) => fb.data(),
            Self::_1920x1080(fb) => fb.data(),
        }
    }
}

impl OriginDimensions for SimpleFrameBuffer {
    fn size(&self) -> Size {
        match self {
            Self::_320x240(fb) => fb.size(),
            Self::_640x480(fb) => fb.size(),
            Self::_1280x720(fb) => fb.size(),
            Self::_1920x1080(fb) => fb.size(),
        }
    }
}

impl DrawTarget for SimpleFrameBuffer {
    type Color = Rgb888;
    type Error = core::convert::Infallible;
    
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>> {
        match self {
            Self::_320x240(fb) => fb.draw_iter(pixels),
            Self::_640x480(fb) => fb.draw_iter(pixels),
            Self::_1280x720(fb) => fb.draw_iter(pixels),
            Self::_1920x1080(fb) => fb.draw_iter(pixels),
        }
    }
}

pub struct TestFrameGenerator {
    w: usize,
    h: usize,
    q: u8,
    text: Option<String>
}

impl TestFrameGenerator {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, q: 50, text: None }
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_owned());
        self
    }

    pub fn with_q(mut self, q: u8) -> Self {
        self.q = q;
        self
    }

    pub fn build_jpeg(self) -> Vec<u8> {
        let mut jpeg_buffer = Vec::with_capacity(2048);
        let encoder: Encoder<&mut Vec<u8>> = Encoder::new(&mut jpeg_buffer, self.q);

        //make simple wrapper for frame buffer
        let mut framebuffer = match (self.w, self.h) {
            (320, 240) => SimpleFrameBuffer::new_320x240(),
            (640, 480) => SimpleFrameBuffer::new_640x480(),
            (1280, 720) => SimpleFrameBuffer::new_1280x720(),
            (1920, 1080) => SimpleFrameBuffer::new_1920x1080(),
            _ => panic!("unsupported frame size!"),
        };

        //fill with white 
        framebuffer.fill_solid(&Rectangle::new(Point::zero(), framebuffer.size()), Rgb888::WHITE).unwrap();

        //test pattern color bars
        let stride = self.w / 7;
        let height = self.h * 3 / 4;
        for (i, color) in [
            Rgb888::CSS_RED,
            Rgb888::CSS_YELLOW,
            Rgb888::CSS_GREEN,
            Rgb888::CSS_CYAN,
            Rgb888::CSS_BLUE,
            Rgb888::CSS_MAGENTA,
        ].iter().enumerate() {
            let x = stride * i;
            framebuffer.fill_solid(&Rectangle::new(Point::new(x as i32, 0), Size::new(stride as u32, height as u32)), *color).unwrap();
        }

        //gradient along bottom 
        for x in 0..self.w {
            let xx = (x * 255) as f64 / (self.w - 1) as f64 ;
            let color = Rgb888::new(xx as u8, xx as u8, xx as u8);
            framebuffer.fill_solid(&Rectangle::new(Point::new(x as i32, height as i32), Size::new(1, self.h as u32 - height as u32)), color).unwrap();
        }
      
        //add text to frame
        if let Some(txt) = self.text {
            let mut style = MonoTextStyle::new(&FONT_10X20, Rgb888::WHITE);
            style.set_background_color(Some(Rgb888::BLACK));
            let txt: Text<'_, MonoTextStyle<'_, Rgb888>> = Text::with_alignment(&txt, Point::new((self.w/2) as i32, (self.h/3) as i32), style, Alignment::Center);
            txt.draw(&mut framebuffer).unwrap();
        }

        //encode as jpeg and return buffer
        encoder.encode(framebuffer.data(), self.w as u16, self.h as u16, jpeg_encoder::ColorType::Rgb).unwrap();
        jpeg_buffer
    }
}