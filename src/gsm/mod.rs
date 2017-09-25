extern crate serial;

mod pdu;

use std::io::{self, BufRead, BufReader};
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GSM_SERIAL_PORT: &'static str = "/dev/ttyAMA0";

const TEST_PHONE_NUMBER: &'static str = "+11234567890";

// Phone Modes:
// PDU(0) vs Text mode(1) for SMS = AT+CMGF

// This is the amount of time that the event thread spends waiting for
// responses from the GSM radio. This will bound how long it takes for
// a command to actually get send to the module.
const PORT_TIMEOUT_MS: u64 = 1000;

// Callbacks in Rust:
// Command, CommandResult -> T, Sender<T>

type SerialThreadResult = Result<(), self::serial::Error>;

// trait Command<T: Send> {
//     fn get_bytes(&self) -> &[u8];
//     fn translate_response(&self, resp: String) -> Result<T, ()>;
//     fn callback(&self, resp: T);
// }

trait Callback {
}

struct Command {
    bytes: Vec<u8>,
    write_cr: bool,
    sender: Option<mpsc::Sender<String>>,
}

impl Command {
    pub fn new_at() -> Command {
        Command {
            bytes: "AT".as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    pub fn new_hangup() -> Command {
        // VOICE CALL: END:
        Command {
            bytes: "ATH".as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    pub fn new_dial(number: &str) -> Command {
        // VOICE CALL: BEGIN:
        Command {
            bytes: format!("ATD{};", number).as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    pub fn new_signal_quality() -> Command {
        Command {
            bytes: "AT+CSQ?".as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    pub fn new_operator_select() -> Command {
        Command {
            bytes: "AT+COPS?".as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    pub fn new_network_system_mode() -> Command {
        Command {
            bytes: "AT+CNSMOD?".as_bytes().to_vec(),
            write_cr: true,
            sender: None,
        }
    }

    // Ringing: 2
    // MISSED_CALL: 09:21AM <NUM>
}

#[derive(Debug)]
struct TTYPhone {
    thread_handler: thread::JoinHandle<SerialThreadResult>,
    command_sender: mpsc::Sender<Command>
}

impl TTYPhone {
    pub fn new(serial_port: &str) -> io::Result<TTYPhone> {
        let serial_port_str: String = String::from(serial_port);

        let (send, recv) = mpsc::channel::<Command>();

        let handle = try!(TTYPhone::start_listener(recv, serial_port_str));

        let phone = TTYPhone {
            thread_handler: handle,
            command_sender: send,
        };

        Ok(phone)
    }

    pub fn send_command(&self, cmd: Command) -> Result<(), mpsc::SendError<Command>> {
        self.command_sender.send(cmd)
    }

    fn start_listener(receiver: mpsc::Receiver<Command>,
                      serial_port: String) -> io::Result<thread::JoinHandle<SerialThreadResult>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("gsm_evt".to_string()).spawn(
            move || {
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
                let mut cmd: Option<Command> = None;

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
                                match cmd.and_then(|f| f.sender ) {
                                    Some(sender) => sender.send(response).ok(),
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

                    thread::sleep(Duration::from_millis(100));
                }
            })
    }

    fn configure_serial_port<T: serial::SerialPort>(mut port: &mut T) -> io::Result<()> {
        // Configure the port
        try!(port.reconfigure(&|settings| {
            try!(settings.set_baud_rate(serial::Baud115200));
            Ok(())
        }));

        try!(port.set_timeout(Duration::from_millis(PORT_TIMEOUT_MS)));
        Ok(())
    }

    fn try_read_from_serial_port<T: serial::SerialPort>(mut reader: &mut BufReader<T>) -> io::Result<Vec<u8>> {
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

    fn write_command_to_serial_port<T: serial::SerialPort>(mut port: &mut T, cmd: &Command) -> io::Result<()> {
        println!("Going to send {:?} with cr {:?}", cmd.bytes, cmd.write_cr);
        try!(port.write(cmd.bytes.as_ref()));

        if cmd.write_cr {
            try!(port.write(&[b'\r']));
        }

        Ok(())
    }

    fn exit(self) {
        // disconnect the sender... (if we actuall want to exit)
        println!("{:?}", self.thread_handler.join());
    }
}

pub fn gsm_main() -> io::Result<()> {
    // match serial::open(GSM_SERIAL_PORT) {
    //     Ok(port) => start_on_port(port),
    //     Err(e) => {
    //         println!("failure: {}", e);
    //         Ok(())
    //     }
    // }

    match TTYPhone::new(GSM_SERIAL_PORT) {
        Ok(phone) => {
            println!("started");

            phone.send_command(Command::new_at()).unwrap();

            let (send, recv) = mpsc::channel();

            phone.send_command(Command {
                bytes: "AT+CMGR=3".as_bytes().to_vec(),
                write_cr: true,
                sender: Some(send.clone()),
            }).unwrap();

            let response = recv.recv().unwrap();
            println!("got response {}", response);

            let mut lines = response.lines();
            lines.next().unwrap();
            println!("parsing response {:?}", pdu::Message::from_string(lines.next().unwrap().to_string()));

            phone.send_command(Command {
                bytes: "AT+CMGL=4".as_bytes().to_vec(),
                write_cr: true,
                sender: Some(send.clone()),
            }).unwrap();

            println!("got response {}", recv.recv().unwrap());

            // phone.send_command(Command::new_dial(TEST_PHONE_NUMBER)).unwrap();

            // thread::sleep(Duration::from_millis(30000));

            // phone.send_command(Command::new_hangup()).unwrap();

            phone.exit();
            Ok(())
        },
        Err(e) => Err(e)
    }
}
