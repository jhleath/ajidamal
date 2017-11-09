#![deny(warnings)]
#![allow(dead_code)]

extern crate rusttype;
extern crate chrono;
extern crate ajidamal;
use ajidamal::display::ui::{Interface};

fn main() {
    Interface::new("/dev/fb1".to_string()).exit()
}
