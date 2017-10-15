// Try to isolate the IPC-server stuff in this module since I'm not
// fully convinced that HTTP will be the way to go in the long
// term. Supposedly you have to worry about resource consumption on
// phones, so perhaps spinning up an entire HTTP stack isn't the way
// to go.

extern crate hyper;
extern crate futures;


use self::futures::future::Future;

use self::hyper::{StatusCode};
use self::hyper::server::{Http, Request, Response, Service};

pub struct GsmServer;

impl GsmServer {
    pub fn start() {
        println!("starting server on 127.0.0.1:3000");
        let addr = "127.0.0.1:3000".parse().unwrap();
        let server = Http::new().bind(&addr, || Ok(GsmServer)).unwrap();
        server.run().unwrap();
    }
}

impl Service for GsmServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;

    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut response = Response::new();

        match (req.method(), req.path()) {
            _ => {
                response.set_status(StatusCode::NotFound)
            },
        }

        Box::new(futures::future::ok(response))
    }
}
