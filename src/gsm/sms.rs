extern crate chrono;

use std::collections::HashMap;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gsm;

use nom::IResult;

use self::chrono::prelude::*;

enum AdditionResult {
    Incomplete(PartialMessage),
    Complete(Message)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub sender: String,
    pub time_stamp: DateTime<Utc>,
    pub contents: String
}

fn get_hash_for_message(sender: &str, reference_number: u8) -> String {
    format!("{}:{}", sender, reference_number)
}

struct PartialMessage {
    reference_number: u8,
    sender: String,
    time_stamp: DateTime<Utc>,
    contents: Vec<Option<String>>,
    total_parts: u8,
    found_parts: u8,
}

impl PartialMessage {
    fn new(reference_number: u8, total_parts: u8, part_index: u8, first_part: gsm::pdu::Message) -> PartialMessage {
        let mut responses: Vec<Option<String>> = vec![None; total_parts as usize];

        assert!(1 <= part_index && part_index <= total_parts);
        responses[(part_index - 1) as usize].get_or_insert(first_part.user_data.data);

        PartialMessage {
            reference_number: reference_number,
            sender: first_part.sender.number,
            time_stamp: first_part.time_stamp,
            contents: responses,
            total_parts: total_parts,
            found_parts: 1,
        }
    }

    fn add_part(mut self, part_index: u8, msg: gsm::pdu::Message) -> AdditionResult {
        assert!(1 <= part_index && part_index <= self.total_parts);
        self.contents[(part_index - 1) as usize].get_or_insert(msg.user_data.data);

        self.found_parts += 1;

        if self.total_parts == self.found_parts {
            // Concatenate all parts of the message
            let full_message = self.contents.into_iter().fold(String::new(), |r, val| r + &val.unwrap());

            AdditionResult::Complete(Message{
                sender: self.sender,
                time_stamp: self.time_stamp,
                contents: full_message,
            })
        } else {
            AdditionResult::Incomplete(self)
        }
    }
}

pub enum Request {
    GetMessages { response: mpsc::Sender<Vec<Message>> },
    SendMessage { destination: String, content: String, response: mpsc::Sender<()> }
}

#[derive(Clone, Debug)]
pub struct MessagingPipe(mpsc::Sender<Request>);

impl MessagingPipe {
    pub fn get_messages(&self) -> mpsc::Receiver<Vec<Message>> {
        let (send, recv) = mpsc::channel();
        self.0.send(Request::GetMessages {
            response: send,
        }).unwrap();

        recv
    }

    pub fn send_message(&self, destination: String, content: String) -> mpsc::Receiver<()> {
        let (send, recv) = mpsc::channel();
        self.0.send(Request::SendMessage{
            destination: destination,
            content: content,
            response: send,
        }).unwrap();

        recv
    }
}

pub struct MessagingManager {
    cmd_send: MessagingPipe,
    join_handle: thread::JoinHandle<Result<(), ()>>
}

fn parse_full_message(msg: gsm::responses::SMS) -> Message {
    Message {
        sender: msg.message.sender.number,
        time_stamp: msg.message.time_stamp,
        contents: msg.message.user_data.data
    }
}

fn parse_messages(msgs: Vec<gsm::responses::SMS>) -> Vec<Message> {
    let mut partial_messages: HashMap<String, PartialMessage> = HashMap::new();
    let mut parsed_messages = Vec::new();

    for mut message in msgs.into_iter() {
        match message.message.user_data.header.take() {
            Some(mut header) => match header.concatenated_message.take() {
                Some(concatenated_message) => {
                    let msg_hash = get_hash_for_message(&message.message.sender.number,
                                                        concatenated_message.reference_number);

                    match partial_messages.remove(&msg_hash) {
                        Some(pm) => {
                            assert!(concatenated_message.reference_number == pm.reference_number);
                            match pm.add_part(concatenated_message.sequence_number, message.message) {

                                AdditionResult::Complete(msg) => { parsed_messages.push(msg); },
                                AdditionResult::Incomplete(pm_new) => { partial_messages.insert(msg_hash, pm_new); }
                            }
                        },
                        None => { partial_messages.insert(msg_hash,
                                                          PartialMessage::new(concatenated_message.reference_number,
                                                                              concatenated_message.number_of_messages,
                                                                              concatenated_message.sequence_number,
                                                                              message.message)); }
                    }
                },
                None => parsed_messages.push(parse_full_message(message))
            },
            None => parsed_messages.push(parse_full_message(message)),
        }
    }

    parsed_messages.sort_by(|a, b| { a.time_stamp.partial_cmp(&b.time_stamp).unwrap() });

    for msg in (&parsed_messages).into_iter() {
        println!("found message {:?}", msg);
    }
    println!("left {} messages unparsed", partial_messages.len());

    parsed_messages
}

fn send_sms(pipeline: &gsm::command::Pipeline, destination: String, content: String) {
    // TODO: Presumably, this is the layer that would handle breaking long text messages up into
    // shorter ones (or implementing the serialize-side of the concatenation protocol). Right now
    // let's just crash if you have something meaningful to say.
    assert!(content.len() < 70);

    // TODO: We should look up the SMSC to prepend to the actual SMS message before submitting it to
    // the radio. Presumably this is something that the outer loop can do every so often and pass
    // into this function.

    // TODO: We should support status reports. This would determine whether or not the SMS message
    // was successfully sent.

    let serialized_message = gsm::pdu::MessageSubmit::new_default(/*reject_duplicates=*/false, /*status_report_request=*/false,
                                                                  gsm::pdu::Number::new_international(destination),
                                                                  gsm::pdu::UserData::new_utf16(content))
        .serialize_to_pdu();
    pipeline.send_sms(serialized_message, None).unwrap();
}

struct MessageData {
    messages: Vec<Message>,
}

impl MessagingManager {
    pub fn new(pipeline: gsm::command::Pipeline) -> MessagingManager {
        let (send, recv) = mpsc::channel::<Request>();

        // Just crash if the program didn't start the thread
        // successfully for now.
        let join_handle = MessagingManager::start_daemon(pipeline, recv).unwrap();

        MessagingManager {
            cmd_send: MessagingPipe(send),
            join_handle: join_handle,
        }
    }

    pub fn exit(self) {
        // TODO: disconnect the sender... (if we actually want to exit)
        println!("exited messaging manager {:?}", self.join_handle.join());
    }

    pub fn get_pipe(&self) -> MessagingPipe {
        self.cmd_send.clone()
    }

    fn start_daemon(pipeline: gsm::command::Pipeline,
                    cmd_recv: mpsc::Receiver<Request>) -> io::Result<thread::JoinHandle<Result<(), ()>>> {
        thread::Builder::new().name("aji/sms".to_string()).spawn(
            move || {
                // At the moment, this thread just loops and
                //
                // 1) Checks for new messages in the GSM radio, and
                // 2) Allows application code to request that the loop either return data (like the
                //    messages that we have) or send new messages.
                //
                // At the end of the day, this is supposed to allow the application layer to be
                // totally isolated from the radio layer (this class will effectively serve as a
                // cache, which will be updated whenever messages arrive). Of course, there are much
                // better ways of doing this than the current implementation. We _should_ provide
                // some mechanism to register with the lower radio layer for unsolicited responses
                // (and turn on unsolicited responses from the physical radio whenever a new text
                // message arrives) rather than poll the radio. In theory, this mechanism would
                // provide a separate channel to wait for this code, but unfortunately Rust doesn't
                // appear to yet have a standardized epoll-like select mechanism to wait on multiple
                // channel receives. Of course, there is likely a way to do this without the mpsc
                // channel abstraction but apparently I used too much Golang and can only think in
                // the CSP model now. Woe is me.

                let mut data = MessageData {
                    messages: Vec::new(),
                };

                // Load text messages every ten seconds from the GSM radio.
                let mut iteration = 0;
                let sms_load_frequency = 1000;

                let (load_callback, load_response) = mpsc::channel();
                let mut waiting_for_load = false;

                loop {
                    iteration = (iteration + 1) % sms_load_frequency;
                    if !waiting_for_load {
                        // Send-side of the loop

                        // Attempt to send a list_sms message
                        if iteration == 0 {
                            match pipeline.list_sms(gsm::command::SMSStore::All, Some(load_callback.clone())) {
                                Ok(_) => (),
                                Err(a) => {
                                    println!("received error sending list sms command {:?}, quitting", a);
                                    return Err(());
                                }
                            };
                            waiting_for_load = true;
                        }

                        // Check the request queue to see if there is anything to process
                        match cmd_recv.try_recv() {
                            Ok(Request::GetMessages{
                                response,
                            }) => {
                                response.send(data.messages.to_vec()).unwrap();
                            },
                            Ok(Request::SendMessage{
                                destination,
                                content,
                                response
                            }) => {
                                send_sms(&pipeline, destination, content);
                                response.send(()).unwrap();
                            },
                            Err(_) => (),
                        };

                    } else {
                        match load_response.try_recv() {
                            Ok((typ, response)) => {
                                assert!(typ == gsm::command::CommandType::ListSMS);
                                waiting_for_load = false;

                                match gsm::responses::parse_list_sms_response(response.as_bytes()) {
                                    IResult::Done(_, list_output) => {
                                        if list_output.code != gsm::responses::ResponseCode::Ok {
                                            return Err(())
                                        }

                                        // Update the data with the parsed messages
                                        data.messages = parse_messages(list_output.sms);
                                    },
                                    a => {
                                        println!("received error parsing the sms messages {:?}", a);
                                        println!("response: {}", response);
                                    }
                                }

                                // TODO: Store the messages somewhere nonvolatile so that a crash in
                                // this program won't require us to reparse the same messages from
                                // the modem.

                                // TODO: Delete the message after it is parsed and stored
                                // successfully. One day, the hope is to durably store these things
                                // on S3, but it looks like disk storage will have to be good for
                                // now before I get TCP over the modem working.
                            },
                            Err(_) => (),
                        }
                    }

                    thread::sleep(Duration::from_millis(10));
                };
            })
    }
}
