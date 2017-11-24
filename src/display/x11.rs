extern crate x11;

use super::{Screen};
use super::base::*;

use std::{ffi, ptr, thread, time};
use self::x11::xlib::*;

pub struct XScreen {}

impl XScreen {
    pub fn new() -> XScreen {
        println!("Starting new XScreen...");

        XScreen {}
    }
}

impl Screen for XScreen {
    fn dimensions(&self) -> (u32, u32) {
        (128, 160)
    }

    fn write_pixel(&mut self, _x: usize, _y: usize, _color: Color) {

    }

    fn flush(&mut self) {

    }
}
