extern crate serial;

use self::serial::SerialPort;

use std::io::{self, BufRead, BufReader, Write};
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GSM_SERIAL_PORT: &'static str = "/dev/ttyAMA0";

const TEST_AD: &'static str = "AT";

// This is the amount of time that the event thread spends waiting for
// responses from the GSM radio. This will bound how long it takes for
// a command to actually get send to the module.
const PORT_TIMEOUT_MS: u64 = 1000;

type SerialThreadResult = Result<(), self::serial::Error>;

struct Command {}

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

    fn start_listener(receiver: mpsc::Receiver<Command>, serial_port: String) -> io::Result<thread::JoinHandle<SerialThreadResult>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("gsm_evt".to_string()).spawn(
            || {
                // TODO: [hleath 2017-09-24] This thread really
                // shouldn't ever return with an error. It should
                // attempt to recover or take the program down.
                
                // Open the serial port
                let mut port = match serial::open(GSM_SERIAL_PORT) {
                    Ok(port) => port,
                    Err(e) => return Err(e),
                };

                // Configure the port
                try!(port.reconfigure(&|settings| {
                    try!(settings.set_baud_rate(serial::Baud115200));
                    Ok(())
                }));
                try!(port.set_timeout(Duration::from_millis(PORT_TIMEOUT_MS)));
                
                let mut reader = BufReader::new(port);
                
                loop {
                    try!(reader.get_mut().write(&[b'A', b'T', 13]));
                    
                    let mut response_buffer: Vec<u8> = Vec::new();
                    
                    match reader.read_until(b'\r', &mut response_buffer) {
                        Ok(_) => println!("{}", String::from_utf8(response_buffer).expect("Invalid UTF-8")),
                        Err(e) => println!("{}", e)
                    }
                    
                    thread::sleep(Duration::from_millis(1000));
                }
            })
    }

    fn wait(mut self) {
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
            phone.wait();
            Ok(())
        },
        Err(e) => Err(e)
    }
}

// fn start_on_port<T: serial::SerialPort>(mut port: T) -> io::Result<()> {

//     let mut response_buffer: Vec<u8> = vec![0; 255];

//     let mut reader = BufReader::new(port);

//     write_command(reader.get_mut(), TEST_AD);
//     println!("Got1 {}", try!(read_response(&mut reader)));

//     thread::sleep(Duration::from_millis(100));

//     send_text_message(&mut reader, "this is a message", &mut response_buffer);
//     send_text_message(&mut reader, "this is a second message", &mut response_buffer);
//     // try!(read_response(&mut reader, &mut responseBuffer));
//     Ok(())
// }

// fn send_text_message<T: serial::SerialPort>(mut reader: &mut BufReader<T>, message: &str,
//                                             mut response_buffer: &mut Vec<u8>) -> io::Result<()> {
//     write_command(reader.get_mut(), TEST_TEXT_MESSAGE);
    
//     thread::sleep(Duration::from_millis(100));
    
//     try!(reader.get_mut().write(message.as_bytes()));
    
//     thread::sleep(Duration::from_millis(100));
    
//     try!(reader.get_mut().write(&[26]));
    
//     thread::sleep(Duration::from_millis(500));
    
//     // try!(port.read(&mut buf[..]));
//     println!("Got2 {}", try!(read_response(&mut reader)));
//     thread::sleep(Duration::from_millis(500));
//     println!("Got3 {}", try!(read_response(&mut reader)));
//     thread::sleep(Duration::from_millis(1000));
//     println!("Got4 {}", try!(read_response(&mut reader)));
//     thread::sleep(Duration::from_millis(500));
//     println!("Got5 {}", try!(read_response(&mut reader)));
//     thread::sleep(Duration::from_millis(500));
//     Ok(())
// }

// fn write_command<T: io::Write>(mut writer: T, command: &str) -> io::Result<()> {
//     try!(writer.write(command.as_bytes()));
//     try!(writer.write(&[b'\r']));
//     Ok(())
// }
