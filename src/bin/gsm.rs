#![deny(warnings)]

// TODO: Get rid of the dead code allowance once we get closer to
// feature completeness in the GSM module.
#![allow(dead_code)]

extern crate ajidamal;

use ajidamal::gsm;
use ajidamal::server;

// TODO: Set up logging so that this program doesn't spew to stdout
// for all of its messages.

fn main() {
    match gsm::Radio::new() {
        Ok(phone) => {
            println!("Successfully started radio, starting HTTP server.");
            server::Server::start(phone);
        },
        Err(e) => {
            println!("Received error starting phone {:?}", e);
        }
    }
}
