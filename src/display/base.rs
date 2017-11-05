#[derive(Clone, Copy, Debug)]
pub struct Color {
    red: u64,
    green: u64,
    blue: u64,
    alpha: f32
}

impl Color {
    pub fn transparent() -> Color {
        Self::new_alpha(0, 0, 0, 0.0)
    }

    pub fn new(r: u64, g: u64, b: u64) -> Color {
        Self::new_alpha(r, g, b, /*alpha=*/1.0)
    }

    pub fn new_alpha(r: u64, g: u64, b: u64, a: f32) -> Color {
        assert!(a >= 0.0);
        assert!(a <= 1.0);

        Color {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        }
    }

    pub fn with_opacity(&self, opacity: f32) -> Color {
        Self::new_alpha(self.red, self.green, self.blue, opacity)
    }

    pub fn overlay(&self, other: &Color) -> Color {
        if other.alpha == 0.0 {
            self.clone()
        } else {
            // TODO: [hleath 2017-11-05] Do this faster?
            // TODO: [hleath 2017-11-05] Preserve the alpha channel?
            let alpha = other.alpha;
            let inv_alpha = 1.0 - alpha;
            Self::new(
                ((other.red as f32 * alpha) + (self.red as f32 * inv_alpha)).round() as u64,
                ((other.green as f32 * alpha) + (self.green as f32 * inv_alpha)).round() as u64,
                ((other.blue as f32 * alpha) + (self.blue as f32 * inv_alpha)).round() as u64)
        }
    }

    pub fn gray(intensity: u64) -> Color {
        Self::new_alpha(intensity, intensity, intensity, 1.0)
    }

    pub fn intensities(&self) -> (u64, u64, u64) {
        (self.red, self.green, self.blue)
    }

    pub fn opacity(&self) -> f32 {
        self.alpha
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
