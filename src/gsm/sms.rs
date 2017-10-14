extern crate chrono;

use nom::IResult;
use gsm;
use std::sync::mpsc;
use self::chrono::prelude::*;
use std::collections::HashMap;

enum AdditionResult {
    Incomplete(PartialMessage),
    Complete(Message)
}

#[derive(Debug)]
struct Message {
    sender: String,
    time_stamp: DateTime<Utc>,
    contents: String
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

pub struct MessagingManager {
    pipeline: gsm::command::Pipeline,
    loaded_messages: Vec<Message>,
}

fn parse_full_message(msg: gsm::responses::SMS) -> Message {
    Message {
        sender: msg.message.sender.number,
        time_stamp: msg.message.time_stamp,
        contents: msg.message.user_data.data
    }
}

impl MessagingManager {
    pub fn new(pipeline: gsm::command::Pipeline) -> MessagingManager {
        MessagingManager {
            pipeline: pipeline,
            loaded_messages: Vec::new(),
        }
    }

    fn parse_messages(&mut self, msgs: Vec<gsm::responses::SMS>) {
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
        for msg in parsed_messages.into_iter() {
            println!("found message {:?}", msg);
        }

        println!("left {} messages unparsed", partial_messages.len());
    }

    pub fn load_text_messages(&mut self) -> Result<(), ()> {
        let (send, recv) = mpsc::channel();

        self.pipeline.list_sms(gsm::command::SMSStore::All, Some(send)).unwrap();

        let (typ, response) = recv.recv().unwrap();
        assert!(typ == gsm::command::CommandType::ListSMS);

        match gsm::responses::parse_list_sms_response(response.as_bytes()) {
            IResult::Done(_, list_output) => {
                if list_output.code != gsm::responses::ResponseCode::Ok {
                    return Err(())
                }

                self.parse_messages(list_output.sms);
                Ok(())
            },
            _ => Err(())
        }
    }
}
