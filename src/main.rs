#![feature(trace_macros)]
#![deny(warnings)]

// TODO: Get rid of the dead code allowance once we get closer to
// feature completeness in the GSM module.
#![allow(dead_code)]

#[macro_use]
extern crate nom;

mod gsm;
mod server;

// TODO: Set up logging so that this program doesn't spew to stdout
// for all of its messages.

fn main() {
    // server::GsmServer::start();

    match gsm::Radio::new() {
        Ok(phone) => {
            println!("Successfully started phone.");

            // As of right now, this line should do nothing but wait
            // on threads to join that will never join.
            phone.shutdown();
        },
        Err(e) => {
            println!("Received error starting phone {:?}", e);
        }
    }
}
