extern crate framebuffer;

pub mod base;
pub mod view;

use self::framebuffer::{Framebuffer};

use self::base::*;
use self::view::{View};

#[derive(Debug)]
pub struct Screen {
    device: Framebuffer,
    frame: Vec<u8>,
    line_length: usize,
    bytes_per_pixel: usize,
    width: u32,
    height: u32,
    root_view: View,
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
            height: h,
            root_view: View::new(w as u64, h as u64)
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
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

    pub fn flush(&mut self) {
        let _ = self.device.write_frame(&self.frame);
    }
}
