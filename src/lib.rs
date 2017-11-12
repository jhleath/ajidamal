#![deny(warnings)]
#![allow(dead_code)]

#[macro_use]
extern crate nom;

#[macro_use]
extern crate serde_derive;

pub mod core;
pub mod display;
pub mod gsm;

// TODO: [hleath 2017-11-02] Build these libraries into the main
// ajidamal library. Right now, they require extra extern crate
// definitions that I'm not moving here.
//
// pub mod server;
