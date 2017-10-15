#![feature(trace_macros)]
#![deny(warnings)]

// TODO: Get rid of the dead code allowance once we get closer to
// feature completeness in the GSM module.
#![allow(dead_code)]

#[macro_use]
extern crate nom;

mod gsm;
mod server;

fn main() {
    server::GsmServer::start();

    match gsm::gsm_main() {
        Ok(_) => println!("it worked"),
        Err(e) => println!("got error {:?}", e)
    }
}
