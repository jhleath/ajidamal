extern crate chrono;

use nom::IResult;
use gsm;
use std::sync::mpsc;
use self::chrono::prelude::*;

enum AdditionResult {
    Incomplete(PartialMessage),
    Complete(Message)
}

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

        assert!(part_index < total_parts);
        responses[part_index as usize].get_or_insert(first_part.user_data.data);

        PartialMessage {
            reference_number: reference_number,
            sender: first_part.sender.number,
            time_stamp: first_part.time_stamp,
            contents: responses,
            total_parts: total_parts,
            found_parts: 1,
        }
    }

    fn get_hash_string(&self) -> String {
        get_hash_for_message(&self.sender, self.reference_number)
    }

    fn add_part(mut self, msg: gsm::pdu::Message) -> AdditionResult {
        let part_index = msg.user_data.header.unwrap().concatenated_message.unwrap().sequence_number;

        self.contents[part_index as usize].get_or_insert(msg.user_data.data);
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
    loaded_messages: Vec<Message>
}

impl MessagingManager {
    pub fn new(pipeline: gsm::command::Pipeline) -> MessagingManager {
        MessagingManager {
            pipeline: pipeline,
            loaded_messages: Vec::new(),
        }
    }

    fn parse_messages(&mut self, msgs: Vec<gsm::responses::SMS>) {

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
