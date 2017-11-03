#![deny(warnings)]
#![allow(dead_code)]

extern crate framebuffer;
extern crate rusttype;
extern crate chrono;

use rusttype::{FontCollection, Scale, point, PositionedGlyph};
use framebuffer::{Framebuffer};
use chrono::prelude::*;

#[derive(Clone, Copy, Debug)]
struct Point {
    x: u64,
    y: u64
}

impl Point {
    pub fn origin() -> Point {
        Self::new(0, 0)
    }

    pub fn new(x: u64, y: u64) -> Point {
        Point {
            x: x,
            y: y,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    origin: Point,

    width: u64,
    height: u64
}

impl Rect {
    pub fn new(o: Point, w: u64, h: u64) -> Rect {
        Rect {
            origin: o,
            width: w,
            height: h
        }
    }

    pub fn from_origin(w: u64, h: u64) -> Rect {
        Self::new(Point::origin(), w, h)
    }
}

#[derive(Debug)]
struct ViewBuffer {
    // TODO: [hleath 2017-11-02] This vector should probably just be
    // u8 so that we can memcpy directly into the frame buffer memory
    // later.
    data: Vec<Color>,

    width: u64,
    height: u64
}

impl ViewBuffer {
    pub fn new(width: u64, height: u64) -> ViewBuffer {
        ViewBuffer {
            width: width,
            height: height,
            data: vec![Color::transparent(); (width * height) as usize]
        }
    }

    pub fn render_full(&mut self, buffer: &ViewBuffer, frame: Rect) {
        let (width, height) = (buffer.width, buffer.height);
        self.render(buffer, frame, Rect::from_origin(width, height));
    }

    pub fn render(&mut self, buffer: &ViewBuffer, frame: Rect, bounds: Rect) {
        // TODO: [hleath 2017-11-02] This code is almost certainly
        // super-slow. I'll need to work to improve it.
        //
        // - Should we use an actual buffer type that allows us to memcpy?
        // - Should we do differential rendering so that we don't have
        //   to update the entire screen every <render_frequence>?

        assert!(frame.width == bounds.width);
        assert!(frame.height == bounds.height);

        // Assert that frame will fit
        assert!(frame.origin.x < self.width);
        assert!(frame.origin.y < self.height);
        assert!(frame.origin.x + frame.width < self.width);
        assert!(frame.origin.y + frame.height < self.height);

        // Assert that bounds exists
        assert!(bounds.origin.x < buffer.width);
        assert!(bounds.origin.y < buffer.height);
        assert!(bounds.origin.x + bounds.width < self.width);
        assert!(bounds.origin.y + bounds.height < self.height);

        let mut frame_index = ((self.height * frame.origin.y) + frame.origin.x) as usize;
        let frame_end_index = ((self.height * (frame.origin.y + frame.height))
                               + (frame.origin.x + frame.width)) as usize;
        let mut row_index = 0 as usize;
        let mut bounds_index = ((buffer.height * bounds.origin.y) + bounds.origin.x) as usize;

        while frame_index < frame_end_index {
            // If we have reached the end of this line, move down a
            // row in the frame and the bounds. This is okay since the
            // frame and the bounds have the same sized rectangles.
            if row_index == frame.width as usize {
                frame_index += self.width as usize;
                bounds_index += buffer.width as usize;
                row_index = 0;
            }

            self.data[frame_index] = self.data[frame_index].overlay(&buffer.data[bounds_index]);

            row_index += 1 as usize;
            frame_index += 1 as usize;
            bounds_index += 1 as usize;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Color {
    red: u64,
    green: u64,
    blue: u64,
    alpha: u64
}

impl Color {
    pub fn transparent() -> Color {
        Self::new_alpha(0, 0, 0, 0)
    }

    pub fn new(r: u64, g: u64, b: u64) -> Color {
        Self::new_alpha(r, g, b, /*alpha=*/255)
    }

    pub fn new_alpha(r: u64, g: u64, b: u64, a: u64) -> Color {
        assert!(a < 256);

        Color {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    pub fn overlay(&self, other: &Color) -> Color {
        // TODO: [hleath 2017-11-02] Actually read up on the RGBA
        // color space to figure out how to overlay colors correctly.
        other.clone()
    }

    pub fn gray(intensity: u64) -> Color {
        Self::new_alpha(intensity, intensity, intensity, 255)
    }
}

// Views are objects that can render to the screen. They may only
// render a portion of their contents to a portion of the screen.
//
// Frame - location in superview
// Bounds - locaiton in this view
#[derive(Debug)]
struct View {
    subviews: Vec<View>,
    frame: Rect,
    bounds: Rect,
    buffer: ViewBuffer
}

impl View {
    fn new(w: u64, h: u64) -> View {
        View {
            subviews: Vec::new(),
            frame: Rect::from_origin(w, h),
            bounds: Rect::from_origin(w, h),
            buffer: ViewBuffer::new(w, h)
        }
    }

    pub fn render(&self) -> ViewBuffer {
        // Don't support scaling the view at all
        assert!(self.frame.width == self.bounds.width);
        assert!(self.frame.height == self.bounds.height);

        // TODO: [hleath 2017-11-02] Each view in the hierarchy will
        // create its own buffer to render onto. This is a lot of
        // allocation. Instead, we should probably just have the lower
        // views compute a frame to the top-level and render directly
        // onto that.

        // Render the current layer of view
        let mut buffer = ViewBuffer::new(self.frame.width, self.frame.height);
        buffer.render(&self.buffer, self.frame, self.bounds);

        for view in self.subviews.iter() {
            let subview_buffer = view.render();
            buffer.render_full(&subview_buffer, view.frame);
        }

        buffer
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

    // Save the width and height to variables so that we don't borrow screen.
    let (screen_width, screen_height) = (screen.width, screen.height);

    // This font is not included in the source code of this
    // repository, you should be able to Google and find it.
    let font_data = include_bytes!("../../fonts/source_code_pro.ttf");

    let collection = FontCollection::from_bytes(font_data as &[u8]);
    let font = collection.into_font().unwrap(); // only succeeds if collection consists of one font

    // Desired font pixel height
    let height: f32 = 14.0; // to get 80 chars across (fits most terminals); adjust as desired
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
    let text_width = glyphs.iter().rev()
        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
        .next().unwrap_or(0.0).ceil() as usize;

    println!("width: {}, height: {}", text_width, pixel_height);

    let text_start_x = (screen_width - text_width as u32) - 5;

    // Split the display into a status bar and a main area
    let status_bar_height = 17;
    draw_rectangle(&mut screen, 0, 0,
                   screen_width, status_bar_height,
                   Color::gray(0));
    draw_rectangle(&mut screen, 0, status_bar_height,
                   screen_width, screen_height - status_bar_height,
                   Color::gray(255));

    let box_height = 50;
    let box_color = Color::new(17, 132, 255);

    draw_rectangle(&mut screen, 2, status_bar_height + 2,
                   screen_width - 4, box_height, box_color);
    draw_rectangle(&mut screen, 2, status_bar_height + 2 * 2 + box_height,
                   screen_width - 4, box_height, box_color);
    draw_rectangle(&mut screen, 2, status_bar_height + 2 * 3 + box_height * 2,
                   screen_width - 4, screen_height - (status_bar_height + 2 * 3 + box_height * 2),
                   box_color);

    for g in glyphs {
        if let Some(bb) = g.pixel_bounding_box() {
            g.draw(|x, y, v| {
                // v should be in the range 0.0 to 1.0
                let color = Color::gray((255_f32 * v).round() as u64);

                let x = x as i32 + bb.min.x;
                let y = y as i32 + bb.min.y;
                // There's still a possibility that the glyph clips the boundaries of the bitmap
                if x >= 0 && x < text_width as i32 && y >= 0 && y < pixel_height as i32 {
                    let x = x as usize;
                    let y = y as usize;

                    screen.write_pixel(x + (text_start_x as usize), y + 1, color);
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
