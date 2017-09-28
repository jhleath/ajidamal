#[macro_use]
extern crate nom;

mod gsm;

fn main() {
    println!("Hello, world!");

    match gsm::gsm_main() {
        Ok(_) => println!("it worked"),
        Err(e) => println!("got error {:?}", e)
    }
}
