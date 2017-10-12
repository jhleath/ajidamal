use std::str;

use gsm;
use gsm::errors::Error;
use gsm::pdu::parse_pdu;

use nom;

#[derive(Debug, PartialEq)]
pub enum ResponseCode {
    Ok = 0
}

fn parse_response_code(data: &[u8]) -> Result<ResponseCode, super::errors::Error> {
    if data.len() > 1 {
        Err(Error::ParseError)
    } else {
        match data[0] {
            b'0' => Ok(ResponseCode::Ok),
            _ => Err(Error::ParseError),
        }
    }
}

#[derive(Debug)]
pub enum MessageStatus {
    ReceivedUnread = 0,
    ReceivedRead = 1,
    StoredUnsent = 2,
    StoredSent = 3
}

fn parse_message_status(data: &[u8]) -> Result<MessageStatus, Error> {
    if data.len() > 1 {
        Err(Error::ParseError)
    } else {
        match data[0] {
            b'0' => Ok(MessageStatus::ReceivedUnread),
            b'1' => Ok(MessageStatus::ReceivedRead),
            b'2' => Ok(MessageStatus::StoredUnsent),
            b'3' => Ok(MessageStatus::StoredSent),
            _ => Err(Error::ParseError),
        }
    }
}

#[derive(Debug)]
pub struct SMS {
    index: Option<u32>,
    status: MessageStatus,
    message: gsm::pdu::Message,
}

#[derive(Debug)]
pub struct ReadSMSResponse {
    sms: SMS,
    code: ResponseCode,
}

named!(pub parse_read_sms_response<ReadSMSResponse>, do_parse!(
    tag_s!("+CMGR: ") >>
    status: map_res!(take_until_and_consume!(","), parse_message_status) >>
    alpha: take_until_and_consume!(",") >>
    length: take_until_and_consume!("\n") >>
    pdu: parse_pdu >>
    tag!("\n") >>
    code: map_res!(call!(nom::rest), parse_response_code) >>
    (ReadSMSResponse {
        sms: SMS {
            index: None,
            status: status,
            message: pdu,
        },
        code: code,
    })
));

#[derive(Debug)]
pub struct ListSMSResponse {
    pub sms: Vec<SMS>,
    pub code: ResponseCode,
}

fn hex_to_u32(data: &[u8]) -> Result<u32, Error> {
    str::from_utf8(data).or(Err(Error::ParseError)).and_then(|s| {
        u32::from_str_radix(s, 16).or(Err(Error::ParseError))
    })
}

named!(pub parse_individual_sms_from_list<SMS>, do_parse!(
    tag_s!("+CMGL: ") >>
    index: map_res!(take_until_and_consume!(","), hex_to_u32) >>
    status: map_res!(take_until_and_consume!(","), parse_message_status) >>
    alpha: take_until_and_consume!(",") >>
    length: take_until_and_consume!("\n") >>
    pdu: parse_pdu >>
    tag!("\n") >>
    (SMS {
        index: Some(index),
        status: status,
        message: pdu,
    })
));

named!(pub parse_list_sms_response<ListSMSResponse>, do_parse!(
    smses: many1!(parse_individual_sms_from_list) >>
    code: map_res!(call!(nom::rest), parse_response_code) >>
    (ListSMSResponse {
        sms: smses,
        code: ResponseCode::Ok,
    })
));
