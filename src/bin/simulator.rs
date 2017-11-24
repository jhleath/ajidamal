#![deny(warnings)]
#![allow(dead_code)]

extern crate ajidamal;

#[cfg(not(feature = "simulator"))]
fn main () {
    println!("The Ajidamal simulator will only work with the `simulator` feature in Cargo.");
}

#[cfg(feature = "simulator")]
fn main() {
    use ajidamal::display::ui::{Interface, ScreenFactory};
    use ajidamal::core::{Core};

    let interface = Interface::new_factory(ScreenFactory::Simulator);
    Core::new(interface).exit();
}
