#![deny(warnings)]
#![allow(dead_code)]

extern crate framebuffer;
extern crate rusttype;
extern crate chrono;

use rusttype::{FontCollection, Scale, point, PositionedGlyph};
use framebuffer::{Framebuffer};
use chrono::prelude::*;

#[derive(Clone, Copy)]
struct Color {
    red: u64,
    green: u64,
    blue: u64
}

impl Color {
    pub fn gray(intensity: u64) -> Color {
        Color {
            red: intensity,
            green: intensity,
            blue: intensity
        }
    }
}

#[derive(Debug)]
struct Screen {
    device: framebuffer::Framebuffer,
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
            height: h,
        }
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel_index = (y * self.line_length) + (x * self.bytes_per_pixel);
        if pixel_index >= self.frame.len() {
            panic!("Aborting because pixel x: {}, y: {} is outside the bounds of the display (w: {}, h: {})",
                   x, y, self.width, self.height);
        }

        // Pack the color into the bytes that we have
        let mut pixel: u64 = 0
            | ((color.red >> (8 - self.device.var_screen_info.red.length)) << self.device.var_screen_info.red.offset)
            | ((color.green >> (8 - self.device.var_screen_info.green.length)) << self.device.var_screen_info.green.offset)
            | ((color.blue >> (8 - self.device.var_screen_info.blue.length)) << self.device.var_screen_info.blue.offset);

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

fn main() {
    let mut screen = Screen::new("/dev/fb1".to_string());

    // This font is not included in the source code of this
    // repository, you should be able to Google and find it.
    let font_data = include_bytes!("../../fonts/source_code_pro.ttf");
    
    let collection = FontCollection::from_bytes(font_data as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

    // Desired font pixel height
    let height: f32 = 12.0; // to get 80 chars across (fits most terminals); adjust as desired
    let pixel_height = height.ceil() as usize;

    // 2x scale in x direction to counter the aspect ratio of monospace characters.
    let scale = Scale { x: height, y: height };

    // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
    // We don't want to clip the text, so we shift it down with an offset when laying it out.
    // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
    // the font. That's enough to guarantee that there's no clipping.
    let v_metrics = font.v_metrics(scale);
    let offset = point(3.0, v_metrics.ascent);

    let local: DateTime<Local> = Local::now();
    
    // Glyphs to draw for "RustType". Feel free to try other strings.
    let glyphs: Vec<PositionedGlyph> = font.layout(
        &format!("{}", local.format("%H:%M %p")), scale, offset).collect();

    // Find the most visually pleasing width to display
    let width = glyphs.iter().rev()
        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
        .next().unwrap_or(0.0).ceil() as usize;

    println!("width: {}, height: {}", width, pixel_height);

    // Split the display into a status bar and a main area
    let status_bar_height = 15;
    let (w, h) = (screen.width, screen.height);
    draw_rectangle(&mut screen, 0, 0, w, status_bar_height, Color::gray(0));
    draw_rectangle(&mut screen, 0, status_bar_height, w, h - status_bar_height, Color::gray(255));

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let color = Color::gray((255_f32 * v).round() as u64);
                
                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;

                    screen.write_pixel(x, y, color);
                }
            })
        }

        screen.flush()
    }
}

fn draw_rectangle(screen: &mut Screen, x: u32, y: u32, w: u32, h: u32, color: Color) {
    // TODO: [2017-11-01] hleath: Implement a blit operation to
    // mass-copy bytes into the mmap'd frame buffer. It is super
    // inefficient to loop about like this.
    
    let mut horiz = x as usize;
    while horiz < (x + w) as usize {
        let mut vert = y as usize;
        
        while vert < (y + h) as usize {
            screen.write_pixel(horiz, vert, color);

            vert += 1;
        }

        horiz += 1;
    }

    screen.flush()
}
