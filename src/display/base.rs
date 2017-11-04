
#[derive(Clone, Copy, Debug)]
pub struct Color {
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

    pub fn intensities(&self) -> (u64, u64, u64) {
        (self.red, self.green, self.blue)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: u64,
    pub y: u64
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
pub struct Rect {
    pub origin: Point,

    pub width: u64,
    pub height: u64
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
