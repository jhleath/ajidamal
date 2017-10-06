use gsm;
use gsm::errors::Error;
use gsm::pdu::parse_pdu;

use nom;

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

pub struct SMS {
    status: MessageStatus,
    message: gsm::pdu::Message,
}

pub struct ReadSMSResponse {
    sms: SMS,
    code: ResponseCode,
}

named!(parse_read_sms_response<ReadSMSResponse>, do_parse!(
    tag_s!("+CMGR: ") >>
    status: map_res!(take_until_and_consume!(","), parse_message_status) >>
    alpha: take_until_and_consume!(",") >>
    length: take_until_and_consume!("\n") >>
    pdu: parse_pdu >>
    tag!("\n") >>
    code: map_res!(call!(nom::rest), parse_response_code) >>
    (ReadSMSResponse {
        sms: SMS {
            status: status,
            message: pdu,
        },
        code: code,
    })
));

pub struct ListSMSResponse {
    sms: Vec<SMS>,
    code: ResponseCode,
}
