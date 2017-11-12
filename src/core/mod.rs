extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration};

use display::ui::{Command, Interface};
use gsm::sms::{Message};

use self::futures::{Future, Stream};
use self::hyper::{Chunk, Client, Error};
use self::tokio_core::reactor;

const CORE_THREAD_SLEEP_MS: u64 = 10000;

pub struct Core {
    interface: Interface,
    thread_handler: thread::JoinHandle<()>
}

impl Core {
    pub fn new(interface: Interface) -> Core {
        let sender = interface.sender.clone();
        Core {
            interface: interface,
            thread_handler: Self::start_thread(sender).unwrap(),
        }
    }

    pub fn exit(self) {
        // TODO: Shut down the interface
        println!("{:?}", self.thread_handler.join());
    }

    // The UI should run on a separate thread from the application
    // logic so that it stays somewhat responsive.
    pub fn start_thread(sender: mpsc::Sender<Command>) -> io::Result<thread::JoinHandle<()>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("aji/core".to_string()).spawn(
            move || {
                loop {
                    let messages = get_messages().unwrap();
                    sender.send(Command::SetMessages(messages)).unwrap();
                    thread::sleep(Duration::from_millis(CORE_THREAD_SLEEP_MS));
                }
            })
    }
}

fn get_messages() -> Result<Vec<Message>, Error> {
    let mut core = reactor::Core::new()?;
    let client = Client::new(&core.handle());

    let uri = "http://127.0.0.1:3000/messages".parse()?;
    let work = client.get(uri).and_then(|res| {
        println!("Response: {}", res.status());

        res.body().concat2().and_then(move |body: Chunk| {
            let messages: Vec<Message> = serde_json::from_slice(&body).unwrap();
            Ok(messages)
        })
    });

    core.run(work)
}
