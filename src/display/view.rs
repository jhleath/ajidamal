use super::base::*;

use std::fmt::Debug;

pub trait Delegate : Debug {
    fn needs_redraw(&self) -> bool;
    fn draw(&mut self, view: &mut View);
}

#[derive(Debug)]
struct NoopDelegate{}

impl Delegate for NoopDelegate {
    fn needs_redraw(&self) -> bool { false }
    // fn handle_event(&self, Event) -> bool {}
    // fn subviews() -> Vec<Box<Delegate>> <-- I don't know about this one.
    fn draw(&mut self, _view: &mut View) { }
}

#[derive(Debug)]
pub struct Buffer {
    // TODO: [hleath 2017-11-02] This vector should probably just be
    // u8 so that we can memcpy directly into the frame buffer memory
    // later.
    data: Vec<Color>,

    width: u64,
    height: u64
}

impl Buffer {
    pub fn new(width: u64, height: u64) -> Buffer {
        Buffer {
            width: width,
            height: height,
            data: vec![Color::transparent(); (width * height) as usize]
        }
    }

    pub fn deconstruct(&self) -> (u64, u64, &[Color]) {
        (self.width, self.height, self.data.as_ref())
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        // Assert dimensions are in bounds
        assert!(x < self.width as usize);
        assert!(y < self.height as usize);

        let index = (y * (self.width as usize)) + x;
        self.data[index] = color;
    }

    fn render_full(&mut self, buffer: &Buffer, frame: Rect) {
        let (width, height) = (buffer.width, buffer.height);
        self.render(buffer, frame, Rect::from_origin(width, height));
    }

    fn render(&mut self, buffer: &Buffer, frame: Rect, bounds: Rect) {
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
        assert!(frame.origin.x + frame.width <= self.width);
        assert!(frame.origin.y + frame.height <= self.height);

        // Assert that bounds exists
        assert!(bounds.origin.x < buffer.width);
        assert!(bounds.origin.y < buffer.height);
        assert!(bounds.origin.x + bounds.width <= self.width);
        assert!(bounds.origin.y + bounds.height <= self.height);

        let mut frame_index = ((self.width * frame.origin.y) + frame.origin.x) as usize;
        let mut bounds_index = ((buffer.width * bounds.origin.y) + bounds.origin.x) as usize;

        let mut column_index = 0 as usize;
        let mut row_index = 0 as usize;

        let mut pixel_index = 0;

        while pixel_index < frame.width * frame.height {
            // If we have reached the end of this line, move down a
            // row in the frame and the bounds. This is okay since the
            // frame and the bounds have the same sized rectangles.
            if column_index == frame.width as usize {
                row_index += 1;

                frame_index = ((self.width as usize) * (row_index + frame.origin.y as usize))
                    + frame.origin.x as usize;
                bounds_index = ((buffer.width as usize) * (row_index + bounds.origin.y as usize))
                    + bounds.origin.x as usize;
                column_index = 0;
            }

            self.data[frame_index] = self.data[frame_index].overlay(&buffer.data[bounds_index]);

            column_index += 1 as usize;
            frame_index += 1 as usize;
            bounds_index += 1 as usize;

            pixel_index += 1;
        }

        println!("rendered {} pixels for {:?} {:?}", pixel_index, frame, bounds)
    }
}

#[derive(Debug)]
pub struct View<'a> {
    bounds: Rect,
    buffer: &'a mut Buffer,
}

impl<'a> View<'a> {
    pub fn new(bounds: Rect, buffer: &'a mut Buffer) -> View {
        assert!(bounds.origin.x < buffer.width);
        assert!(bounds.origin.x + bounds.width <= buffer.width);
        assert!(bounds.origin.y < buffer.height);
        assert!(bounds.origin.y + bounds.height <= buffer.height);

        View {
            bounds: bounds,
            buffer: buffer,
        }
    }

    pub fn width(&self) -> u64 {
        self.bounds.width
    }

    pub fn height(&self) -> u64 {
        self.bounds.height
    }

    pub fn new_full(buffer: &'a mut Buffer) -> View {
        let bounds = Rect::from_origin(buffer.width, buffer.height);
        Self::new(bounds, buffer)
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        assert!((x as u64) < self.bounds.width);
        assert!((y as u64) < self.bounds.height);

        self.buffer.write_pixel(x + (self.bounds.origin.x as usize),
                                y + (self.bounds.origin.y as usize),
                                color)
    }
}
