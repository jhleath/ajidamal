extern crate serial;

use std::io::{self, BufRead, BufReader};
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GSM_SERIAL_PORT: &'static str = "/dev/ttyAMA0";

const TEST_PHONE_NUMBER: &'static str = "+11234567890";

// This is the amount of time that the event thread spends waiting for
// responses from the GSM radio. This will bound how long it takes for
// a command to actually get send to the module.
const PORT_TIMEOUT_MS: u64 = 1000;

type SerialThreadResult = Result<(), self::serial::Error>;

struct Command {
    string: String,
    write_cr: bool,
}

impl Command {
    pub fn new_at() -> Command {
        Command {
            string: "AT".to_string(),
            write_cr: true
        }
    }

    pub fn new_hangup() -> Command {
        Command {
            string: "ATH".to_string(),
            write_cr: true
        }
    }

    pub fn new_dial(number: &str) -> Command {
        Command {
            string: format!("ATD{};", number),
            write_cr: true
        }
    }
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

                loop {
                    // First try to get a command from the command
                    // channel:
                    match receiver.try_recv() {
                        Ok(cmd) => {
                            try!(Self::write_command_to_serial_port(reader.get_mut(), cmd));
                        }, // issue command
                        Err(mpsc::TryRecvError::Empty) => {}, // Nothing to do
                        Err(mpsc::TryRecvError::Disconnected) => {
                            return Ok(())
                        }
                    }

                    match Self::try_read_from_serial_port(&mut reader) {
                        Ok(data) => println!("received: {}", String::from_utf8(data).expect("Invalid UTF-8")),
                        Err(_) => {
                            // Read likely failed because of timeout, do nothing.
                            ()
                        }
                    };

                    thread::sleep(Duration::from_millis(1000));
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
            Ok(_) => Ok(response_buffer),
            Err(e) => Err(e)
        }
    }

    fn write_command_to_serial_port<T: serial::SerialPort>(mut port: &mut T, cmd: Command) -> io::Result<()> {
        println!("Going to send {} with cr {:?}", cmd.string, cmd.write_cr);
        try!(port.write(cmd.string.as_bytes()));

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

            phone.send_command(Command::new_dial(TEST_PHONE_NUMBER)).unwrap();

            thread::sleep(Duration::from_millis(30000));

            phone.send_command(Command::new_hangup()).unwrap();

            phone.exit();
            Ok(())
        },
        Err(e) => Err(e)
    }
}
