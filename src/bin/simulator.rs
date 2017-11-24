#![deny(warnings)]
#![allow(dead_code)]

extern crate ajidamal;

use ajidamal::display::ui::{Interface, ScreenFactory};
use ajidamal::core::{Core};

#[cfg(not(feature = "simulator"))]
fn main () {
    println!("The Ajidamal simulator will only work with the `simulator` feature in Cargo.");
}

#[cfg(feature = "simulator")]
fn main() {
    let interface = Interface::new_factory(ScreenFactory::Simulator);
    Core::new(interface).exit();
}
