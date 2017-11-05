use super::base::*;

#[derive(Debug)]
pub struct ViewBuffer {
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

    pub fn deconstruct(self) -> (u64, u64, Vec<Color>) {
        (self.width, self.height, self.data)
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        // Assert dimensions are in bounds
        assert!(x < self.width as usize);
        assert!(y < self.height as usize);

        let index = (y * (self.width as usize)) + x;
        self.data[index] = color;
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

// Views are objects that can render to the screen. They may only
// render a portion of their contents to a portion of the screen.
//
// Frame - location in superview
// Bounds - locaiton in this view
#[derive(Debug)]
pub struct View {
    subviews: Vec<View>,
    frame: Rect,
    bounds: Rect,
    buffer: ViewBuffer
}

impl View {
    pub fn new(w: u64, h: u64) -> View {
        View {
            subviews: Vec::new(),
            frame: Rect::from_origin(w, h),
            bounds: Rect::from_origin(w, h),
            buffer: ViewBuffer::new(w, h)
        }
    }

    pub fn height(&self) -> u64 {
        self.buffer.height
    }

    pub fn width(&self) -> u64 {
        self.buffer.width
    }

    pub fn add_full_subview(&mut self, subview: View, origin: Point) {
        let (width, height) = (subview.width(), subview.height());
        self.add_subview(subview,
                         Rect::new(origin, width, height),
                         Rect::from_origin(width, height));
    }

    pub fn add_subview(&mut self, mut subview: View, frame: Rect, bounds: Rect) {
        assert!(frame.width == bounds.width);
        assert!(frame.height == bounds.height);

        subview.frame = frame;
        subview.bounds = bounds;
        self.subviews.push(subview);
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        self.buffer.write_pixel(x, y, color)
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
        buffer.render(&self.buffer,
                      Rect::from_origin(self.frame.width, self.frame.height),
                      self.bounds);

        for view in self.subviews.iter() {
            let subview_buffer = view.render();
            buffer.render_full(&subview_buffer, view.frame);
        }

        buffer
    }
}
