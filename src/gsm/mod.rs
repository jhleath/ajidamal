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

#[derive(Debug)]
struct TTYPhone {
    thread_handler: thread::JoinHandle<SerialThreadResult>,
    command_sender: mpsc::Sender<command::RawCommand>
}

impl TTYPhone {
    pub fn new(serial_port: &str) -> io::Result<TTYPhone> {
        let serial_port_str: String = String::from(serial_port);

        let (send, recv) = mpsc::channel::<command::RawCommand>();

        let handle = try!(TTYPhone::start_listener(recv, serial_port_str));

        let phone = TTYPhone {
            thread_handler: handle,
            command_sender: send,
        };

        Ok(phone)
    }

    pub fn send_command(&self, cmd: command::RawCommand) -> Result<(), mpsc::SendError<command::RawCommand>> {
        self.command_sender.send(cmd)
    }

    fn start_listener(receiver: mpsc::Receiver<command::RawCommand>,
                      serial_port: String) -> io::Result<thread::JoinHandle<SerialThreadResult>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("gsm_evt".to_string()).spawn(
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
                            if data[0] != 10 {
                                response = String::new();
                            }

                            response += &String::from_utf8(data).expect("Invalid UTF-8")
                        },

                        Err(e) => if e.kind() == io::ErrorKind::TimedOut {
                            if response.len() > 0 {
                                // Send response back to the Command.
                                match cmd.and_then(|f| f.get_callback() ) {
                                    Some((command_type, sender)) => sender.send((command_type, response)).ok(),
                                    None => {
                                        println!("received unsolicited response {}", response);
                                        None
                                    },
                                };

                                response = String::new();
                                cmd = None
                            }

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
        // TODO: disconnect the sender... (if we actuall want to exit)
        println!("{:?}", self.thread_handler.join());
    }
}

pub fn gsm_main() -> io::Result<()> {
    match TTYPhone::new(GSM_SERIAL_PORT) {
        Ok(phone) => {
            // Send AT just to be sure that things are working.
            let pipeline = command::Pipeline::new(phone.command_sender.clone());
            pipeline.attention(None).unwrap();

            // let (send, recv) = mpsc::channel();

            // pipeline.list_sms(command::SMSStore::All, Some(send)).unwrap();

            // let (typ, response) = recv.recv().unwrap();
            // assert!(typ == command::CommandType::ListSMS);
            // println!("got response {:?}:{}", typ,response);
            // println!("parsing response {:?}", responses::parse_list_sms_response(response.as_bytes()));

            let mut sms_manager = sms::MessagingManager::new(pipeline);
            sms_manager.load_text_messages().unwrap();

            phone.exit();
            Ok(())
        },
        Err(e) => Err(e)
    }
}
