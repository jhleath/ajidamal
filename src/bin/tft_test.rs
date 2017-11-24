#![deny(warnings)]
#![allow(dead_code)]

extern crate rusttype;
extern crate chrono;
extern crate ajidamal;
use ajidamal::display::ui::{Interface};
use ajidamal::core::{Core};

extern crate serde_json;

fn main() {
    let interface = Interface::new("/dev/fb1".to_string());
    Core::new(interface).exit();
}
