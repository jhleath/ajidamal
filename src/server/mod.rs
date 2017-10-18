// Try to isolate the IPC-server stuff in this module since I'm not
// fully convinced that HTTP will be the way to go in the long
// term. Supposedly you have to worry about resource consumption on
// phones, so perhaps spinning up an entire HTTP stack isn't the way
// to go.

extern crate hyper;
extern crate futures;


use self::futures::Stream;
use self::futures::future::Future;

use self::hyper::{Body, Chunk, Method, StatusCode};
use self::hyper::server::{Http, Request, Response, Service};

pub struct Server {
    radio: super::gsm::RadioClient,
}

impl Server {
    pub fn start(radio: super::gsm::Radio) {
        println!("starting server on 127.0.0.1:3000");
        let addr = "0.0.0.0:3000".parse().unwrap();

        let server = Http::new().bind(&addr, move || Ok(Server{
            radio: radio.get_client()
        })).unwrap();

        server.run().unwrap();
    }
}

impl Service for Server {
    type Request = Request;
    type Response = Response<Box<Stream<Item=Chunk, Error=Self::Error>>>;
    type Error = hyper::Error;

    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut response = Response::new();

        match (req.method(), req.path()) {
            (&Method::Get, "/messages") => {
                let messages = self.radio.sms.get_messages().recv().unwrap();
                let body: Box<Stream<Item=_, Error=_>> = Box::new(Body::from(format!("{:?}", messages)));
                response.set_body(body);
            },
            _ => {
                response.set_status(StatusCode::NotFound)
            },
        }

        Box::new(futures::future::ok(response))
    }
}
