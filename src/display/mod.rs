extern crate framebuffer;

pub mod base;
pub mod text;
pub mod view;
pub mod ui;

use self::framebuffer::{Framebuffer};

use self::base::*;
use self::view::{Buffer};

#[derive(Debug)]
pub struct Screen {
    device: Framebuffer,
    frame: Vec<u8>,
    line_length: usize,
    bytes_per_pixel: usize,
    width: u32,
    height: u32,
}

impl Screen {
    pub fn new(device_path: String) -> Screen {
        let device = Framebuffer::new(&device_path).unwrap();

        let w = device.var_screen_info.xres;
        let h = device.var_screen_info.yres;
        let line_length = device.fix_screen_info.line_length;
        let bytespp = device.var_screen_info.bits_per_pixel / 8;

        println!("Started screen device with properties: w:{}, h:{}, line_length:{}, bytespp:{}",
                 w, h, line_length, bytespp);

        Screen {
            device: device,
            frame: vec![0u8; (line_length * h) as usize],
            line_length: line_length as usize,
            bytes_per_pixel: bytespp as usize,
            width: w,
            height: h
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        // We should only be writing fully opaque pixels to the
        // display at this point.
        if color.opacity() != 1.0 {
            return
        }

        let pixel_index = (y * self.line_length) + (x * self.bytes_per_pixel);
        if pixel_index >= self.frame.len() {
            panic!("Aborting because pixel x: {}, y: {} is outside the bounds of the display (w: {}, h: {})",
                   x, y, self.width, self.height);
        }

        // Pack the color into the bytes that we have
        let (red, green, blue) = color.intensities();
        let mut pixel: u64 = 0
            | ((red >> (8 - self.device.var_screen_info.red.length)) << self.device.var_screen_info.red.offset)
            | ((green >> (8 - self.device.var_screen_info.green.length)) << self.device.var_screen_info.green.offset)
            | ((blue >> (8 - self.device.var_screen_info.blue.length)) << self.device.var_screen_info.blue.offset);

        let mut bytes_used = 0;
        while bytes_used < self.bytes_per_pixel {
            // Grabe the lower 8 bits of the pixel
            let pixel_part = (pixel & 255) as u8;
            self.frame[pixel_index + bytes_used] = pixel_part;
            pixel = pixel >> 8;

            bytes_used += 1;
        }
    }

    pub fn render_view(&mut self, buffer: &Buffer) {
        let (width, height, data) = buffer.deconstruct();

        let mut x = 0;
        let mut y = 0;
        while y < height {
            while x < width {
                self.write_pixel(x as usize, y as usize,
                                 data[((y * width) + x) as usize]);
                x += 1;
            }

            x = 0;
            y+= 1;
        }

        self.flush();
    }

    pub fn flush(&mut self) {
        let _ = self.device.write_frame(&self.frame);
    }
}
