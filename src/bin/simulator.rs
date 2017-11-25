#![deny(warnings)]
#![allow(dead_code)]

extern crate ajidamal;

#[macro_use]
extern crate clap;

use clap::{Arg, App};

#[cfg(not(feature = "simulator"))]
fn main () {
    println!("The Ajidamal simulator will only work with the `simulator` feature in Cargo.");
}

#[cfg(feature = "simulator")]
fn main() {
    let matches = App::new("Ajidamal Simulator")
        .arg(Arg::with_name("scale")
             .long("scale")
             .default_value("2")
             .takes_value(true))
        .get_matches();

    use ajidamal::display::ui::{Interface, ScreenFactory};
    use ajidamal::core::{Core};

    let scale = value_t!(matches, "scale", u64).unwrap_or_else(|e| e.exit());
    let interface = Interface::new_factory(ScreenFactory::Simulator(scale));
    Core::new(interface).exit();
}
