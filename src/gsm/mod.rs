extern crate serial;

pub mod sms;
mod pdu;
pub mod command;
mod responses;
mod errors;

use std::io::{self, BufRead, BufReader};
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GSM_SERIAL_PORT: &'static str = "/dev/ttyAMA0";

// This is the amount of time that the event thread spends waiting for
// responses from the GSM radio. This will bound how long it takes for
// a command to actually get send to the module.
const PORT_TIMEOUT_MS: u64 = 1000;

// How long for the event loop thread to sleep in between reads. This
// will bound the input/output speed of the device from this layer
// since the driver will only read one line at a time for now.
const EVT_THREAD_SLEEP_MS: u64 = 10;

type SerialThreadResult = Result<(), self::serial::Error>;

pub type ModemPipe = mpsc::Sender<command::RawCommand>;

#[derive(Debug)]
struct SerialModem {
    thread_handler: thread::JoinHandle<SerialThreadResult>,
    command_sender: ModemPipe
}

impl SerialModem {
    pub fn new(serial_port: &str) -> io::Result<SerialModem> {
        let serial_port_str: String = String::from(serial_port);

        let (send, recv) = mpsc::channel::<command::RawCommand>();

        let handle = try!(SerialModem::start_listener(recv, serial_port_str));

        let phone = SerialModem {
            thread_handler: handle,
            command_sender: send,
        };

        Ok(phone)
    }

    pub fn get_pipe(&self) -> ModemPipe {
        self.command_sender.clone()
    }

    pub fn send_command(&self, cmd: command::RawCommand) -> Result<(), mpsc::SendError<command::RawCommand>> {
        self.command_sender.send(cmd)
    }

    fn start_listener(receiver: mpsc::Receiver<command::RawCommand>,
                      serial_port: String) -> io::Result<thread::JoinHandle<SerialThreadResult>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("aji/gsm_evt".to_string()).spawn(
            move || {
                // TODO: [hleath 2017-09-30] Use Async IO.

                // TODO: [hleath 2017-09-24] This thread really
                // shouldn't ever return with an error. It should
                // attempt to recover or take the program down.

                // Open the serial port
                let mut port = match serial::open(&serial_port) {
                    Ok(port) => port,
                    Err(e) => return Err(e),
                };

                try!(Self::configure_serial_port(&mut port));

                let mut reader = BufReader::new(port);

                let mut response = String::new();
                let mut cmd: Option<command::RawCommand> = None;

                loop {
                    if cmd.is_none() {
                        // First try to get a command from the command
                        // channel:
                        match receiver.try_recv() {
                            Ok(recv_cmd) => {
                                try!(Self::write_command_to_serial_port(reader.get_mut(), &recv_cmd));
                                cmd = Some(recv_cmd);
                            },
                            Err(mpsc::TryRecvError::Empty) => {}, // Nothing to do
                            Err(mpsc::TryRecvError::Disconnected) => {
                                return Ok(())
                            }
                        }
                    }

                    match Self::try_read_from_serial_port(&mut reader) {
                        Ok(data) => {
                            if data.len() > 0 {
                                if data[0] != 10 {
                                    response = String::new();
                                }

                                response += &String::from_utf8(data).expect("Invalid UTF-8")
                            }
                        },

                        Err(e) => if e.kind() == io::ErrorKind::TimedOut {
                            // Send response back to the Command.
                            if response.len() > 0 {
                                match cmd.and_then(|f| f.get_callback() ) {
                                    Some((command_type, sender)) => sender.send((command_type, response)).ok(),
                                    None => {
                                        println!("received unsolicited response {}", response);
                                        None
                                    },
                                };
                            }

                            response = String::new();
                            cmd = None

                            // Without a processing response, there is
                            // nothing to do during a timeout.
                        } else {
                            println!("got other error {}", e);
                        }
                    };

                    // Commands that begin with a line feed (10) are
                    // part of the same response. And a carriage
                    // return (13) is at the end of each line.

                    thread::sleep(Duration::from_millis(EVT_THREAD_SLEEP_MS));
                }
            })
    }

    fn configure_serial_port<T: serial::SerialPort>(port: &mut T) -> io::Result<()> {
        // Configure the port
        try!(port.reconfigure(&|settings| {
            try!(settings.set_baud_rate(serial::Baud115200));
            Ok(())
        }));

        try!(port.set_timeout(Duration::from_millis(PORT_TIMEOUT_MS)));
        Ok(())
    }

    fn try_read_from_serial_port<T: serial::SerialPort>(reader: &mut BufReader<T>) -> io::Result<Vec<u8>> {
        let mut response_buffer: Vec<u8> = Vec::new();

        match reader.read_until(b'\r', &mut response_buffer) {
            Ok(num_bytes) => {
                // Trim off any excess \r
                if response_buffer[num_bytes - 1] == b'\r' {
                    response_buffer = response_buffer[0..num_bytes - 1].to_vec()
                }

                Ok(response_buffer)
            },
            Err(e) => Err(e)
        }
    }

    fn write_command_to_serial_port<T: serial::SerialPort>(port: &mut T, cmd: &command::RawCommand) -> io::Result<()> {
        let sending_bytes = cmd.render();
        println!("Going to send {:?}.", sending_bytes);
        try!(port.write(sending_bytes.as_ref()));

        Ok(())
    }

    fn exit(self) {
        // TODO: disconnect the sender... (if we actually want to exit)
        println!("{:?}", self.thread_handler.join());
    }
}

pub struct Radio {
    phone: SerialModem,
    pub sms: sms::MessagingManager,
}

#[derive(Clone)]
pub struct RadioClient {
    pub phone: ModemPipe,
    pub sms: sms::MessagingPipe,
}

impl Radio {
    pub fn new() -> Result<Radio, errors::Error> {
        match SerialModem::new(GSM_SERIAL_PORT) {
            Ok(phone) => {
                let sms_pipeline = command::Pipeline::new(phone.command_sender.clone());

                // Ensure that the phone is working before returning to caller.
                Radio::synchronous_attention_internal(&sms_pipeline);

                // Immediately start a background worker for
                Ok(Radio {
                    phone: phone,
                    sms: sms::MessagingManager::new(sms_pipeline),
                })
            },
            Err(e) => {
                println!("Error starting SerialModem: {:?}", e);
                Err(errors::Error::LoadError)
            }
        }
    }

    pub fn get_client(&self) -> RadioClient {
        RadioClient {
            phone: self.phone.get_pipe(),
            sms: self.sms.get_pipe(),
        }
    }

    pub fn synchronous_attention(&self) {
        // TODO: Clean this up, we shouldn't need to create a new
        // struct just to send a simple command.
        let temp_pipeline = command::Pipeline::new(self.phone.command_sender.clone());
        Radio::synchronous_attention_internal(&temp_pipeline)
    }

    fn synchronous_attention_internal(pipeline: &command::Pipeline) {
        let (send, recv) = mpsc::channel();
        pipeline.attention(Some(send)).unwrap();

        // Wait 5 seconds to get a response from the attention
        recv.recv_timeout(Duration::from_millis(5000)).unwrap();
    }

    pub fn shutdown(self) {
        self.sms.exit();
        self.phone.exit();
    }
}
