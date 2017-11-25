pub mod base;
pub mod text;
pub mod view;
pub mod ui;

#[cfg(feature = "simulator")]
pub mod x11;

pub mod frame_buffer;

use self::base::*;
use self::view::{Buffer};

pub trait Screen {
    fn dimensions(&self) -> (u32, u32);
    fn write_pixel(&mut self, x: usize, y: usize, color: Color);
    fn flush(&mut self);
}

pub fn render_view(screen: &mut Screen, buffer: &Buffer) {
    let (width, height, data) = buffer.deconstruct();

    let mut x = 0;
    let mut y = 0;
    while y < height {
        while x < width {
            screen.write_pixel(x as usize, y as usize,
                               data[((y * width) + x) as usize]);
            x += 1;
        }

        x = 0;
        y+= 1;
    }

    screen.flush();
}

#[cfg(not(feature = "simulator"))]
pub fn create_simulator(_scale: u64) -> Box<Screen> {
    panic!("Cannot create simulator at runtime without simulator compilation support.")
}

#[cfg(feature = "simulator")]
pub fn create_simulator(scale: u64) -> Box<Screen> {
    Box::new(x11::XScreen::new(scale))
}
