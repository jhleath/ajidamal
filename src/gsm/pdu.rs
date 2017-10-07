extern crate chrono;

use self::chrono::prelude::*;
use super::errors::Error;
use std::str;
use std::num::ParseIntError;
use nom::IResult;
use nom;

#[derive(Debug)]
pub struct Number {
    format: AddressType,
    number: String,
}

#[derive(Debug)]
pub struct UserData {
    encoding: Encoding,
    data: String
}

#[derive(Debug)]
pub struct Message {
    service_center: Number,
    command_type: CommandType,
    sender: Number,
    time_stamp: DateTime<Utc>,
    protocol_id: u8,
    user_data: UserData,
}

#[derive(Debug)]
enum CommandType {
    SmsDeliver,
    VoicemailUpdate
}

#[derive(Debug)]
enum Encoding {
    Gsm7Bit,
    Utf16,
    Unknown
}

#[derive(Debug)]
enum AddressType {
    International, // 145
    ShortCode, // 201
}

fn u16_from_hex_str(data: &[u8]) -> Result<u16, Error> {
    str::from_utf8(data).or(Err(Error::ParseError)).and_then(|s| {
        u16::from_str_radix(s, 16).or(Err(Error::ParseError))
    })
}

fn u8_from_hex_str(data: &[u8]) -> Result<u8, Error> {
    str::from_utf8(data).or(Err(Error::ParseError)).and_then(|s| {
        u8::from_str_radix(s, 16).or(Err(Error::ParseError))
    })
}

fn str_from_decimal_octet(data: &[u8]) -> Result<String, Error> {
    let mut output = String::new();

    output.push(char::from(data[1]));

    if data[0] != b'F' {
        output.push(char::from(data[0]));
    }

    Ok(output)
}

fn concat_strings(data: Vec<String>) -> Result<String, Error> {
    Ok(data.into_iter().fold(String::new(), |acc, item| {
        acc + &item
    }))
}

fn get_decimal_length(data: u8) -> Result<u8, Error> {
    if data % 2 == 0 {
        Ok(data / 2)
    } else {
        Ok((data / 2) + 1)
    }
}

fn to_vec(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(data.to_vec())
}

fn to_command_type(data: u8) -> Result<CommandType, Error> {
    match data {
        4 => Ok(CommandType::SmsDeliver),
        64 => Ok(CommandType::VoicemailUpdate),
        d => {
            println!("got unexpected command type {:?}", d);
            Err(Error::ParseError)
        }
    }
}

fn to_encoding_scheme(data: u8) -> Result<Encoding, Error> {
    match data {
        0 => Ok(Encoding::Gsm7Bit),
        8 => Ok(Encoding::Utf16),
        d => {
            println!("got unexpected encoding scheme {:?}", d);
            Err(Error::ParseError)
        }
    }
}

fn to_address_type(data: u8) -> Result<AddressType, Error> {
    match data {
        145 => Ok(AddressType::International), // International number + ISDN
        201 => Ok(AddressType::ShortCode), // Subscriber  number + private numbering
        d => {
            println!("got unexpected address type {:?}", d);
            Err(Error::ParseError)
        }
    }
}

fn parse_ascii_hex_number(data: u8) -> i32 {
    match data {
        48 => 0,
        49 => 1,
        50 => 2,
        51 => 3,
        52 => 4,
        53 => 5,
        54 => 6,
        55 => 7,
        56 => 8,
        57 => 9,
        // uppercase
        65 => 10,
        66 => 11,
        67 => 12,
        68 => 13,
        69 => 14,
        70 => 15,
        // lowercase
        97 => 10,
        98 => 11,
        99 => 12,
        100 => 13,
        101 => 14,
        102 => 15,
        // failure
        _ => 0,
    }
}

fn parse_time_zone(mut zero: i32, mut one: i32) -> i32 {
    let mut sign = 1;

    // The third bit stores the sign information. If it is 1, then the
    // time zone is negative. Ignore that bit and parse the number as
    // usual.
    if one & 0b1000 != 0 {
        one = one & 0b0111;
        sign = -1;
    }
    
    sign * ((10 * one) + zero)
}

fn parse_date_time(tz_data: &[u8], mut data: String) -> Result<DateTime<Utc>, Error> {
    //yyMMddHHMMss
    let FORMAT_STRING: &str = "%y%m%d%H%M%S";

    let time_zone = parse_time_zone(parse_ascii_hex_number(tz_data[0]), parse_ascii_hex_number(tz_data[1]));

    let datetime = match FixedOffset::east(time_zone * 900)
                                     .datetime_from_str(data.as_ref(), FORMAT_STRING) {
        Ok(d) => d.with_timezone(&Utc),
        Err(e) => {
            println!("Got {:?} parsing the datetime", e);
            return Err(Error::ParseError)
        }
    };

    Ok(datetime)
}

fn parse_user_data(data: &[u8], encoding: Encoding, length: u8) -> IResult<&[u8], UserData> {
    match encoding {
        Encoding::Gsm7Bit => parse_gsm_alphabet(data, length).map(|parsed_data| {
            UserData {
                encoding: Encoding::Gsm7Bit,
                data: parsed_data,
            }
        }),
        Encoding::Utf16 => parse_utf16(data, length as usize).map(|parsed_data| {
            UserData {
                encoding: Encoding::Utf16,
                data: parsed_data,
            }
        }),
        Encoding::Unknown => {
            IResult::Done(&data[length as usize..],
                          UserData {
                              encoding: Encoding::Unknown,
                              data: format!("unknown encoding for data: {:?}", &data[..length as usize]),
                          })
        }
    }
}

named!(hex_octet<u8>, map_res!(take!(2), u8_from_hex_str));

named!(decimal_octet<String>, map_res!(take!(2), str_from_decimal_octet));

named_args!(decimal_octet_number(length: u8)<String>,
            map_res!(
                count!(decimal_octet, length as usize),
                concat_strings));

named!(pub parse_pdu<Message>,
       do_parse!(
           sc_length: hex_octet >>
           sc_address_type: map_res!(hex_octet, to_address_type) >>
           service_center: apply!(decimal_octet_number, sc_length - 1) >>
           message_type: map_res!(hex_octet, to_command_type) >>
           sender_length: map_res!(hex_octet, get_decimal_length) >>
           sender_address_type: map_res!(hex_octet, to_address_type) >>    
           sender: apply!(decimal_octet_number, sender_length) >>
           protocol_id: hex_octet >>
           encoding_scheme: map_res!(hex_octet, to_encoding_scheme) >>
           time_stamp: apply!(decimal_octet_number, 6) >>
           time_zone: take!(2) >>
           ud_length: hex_octet >>
           user_data: apply!(parse_user_data, encoding_scheme, ud_length) >>
               
           (Message {
               service_center: Number {
                   format: sc_address_type,
                   number: service_center,
               },
               command_type: message_type,
               sender: Number {
                   format: sender_address_type,
                   number: sender,
               },
               protocol_id: protocol_id,
               time_stamp: parse_date_time(time_zone, time_stamp).unwrap(),
               user_data: user_data,
           })
       )
);

impl Message {
    pub fn from_string(pdu_string: String) -> Result<Message, ()> {
        match parse_pdu(pdu_string.as_bytes()) {
            IResult::Done(rest, m) => {
                Ok(m)
            },
            IResult::Error(e) => Err(()),
            IResult::Incomplete(n) => {
                println!("incomplete? {:?}", n);
                Err(())
            }
        }
    }
}

const GSM_MASKS: &[u8] = &[
    0b01111111, // 1
    0b00111111, // 2
    0b00011111, // 3
    0b00001111, // 4
    0b00000111, // 5
    0b00000011, // 6
    0b00000001, // 7
    0b11111111, // 0
];

const GSM_CHARS: &[char] = &[
//   0     1     2     3     4     5     6     7     8     9     A     B     C      D    E     F
    '@',  '£',  '$',  '¥',  'è',  'é',  'ù',  'ì',  'ò',  'Ç', '\n',  'Ø',  'ø', '\r',  'Å',  'å', // 0
    'Δ',  '_',  'Φ',  'Γ',  'Λ',  'Ω',  'Π',  'Ψ',  'Σ',  'Θ',  'Ξ',  '?',  'Æ',  'æ',  'ß',  'É', // 1
    ' ',  '!',  '"',  '#',  '¤',  '%',  '&', '\'',  '(',  ')',  '*',  '+',  ',',  '-',  '.',  '/', // 2
    '0',  '1',  '2',  '3',  '4',  '5',  '6',  '7',  '8',  '9',  ':',  ';',  '<',  '=',  '>',  '?', // 3
    '¡',  'A',  'B',  'C',  'D',  'E',  'F',  'G',  'H',  'I',  'J',  'K',  'L',  'M',  'N',  'O', // 4
    'P',  'Q',  'R',  'S',  'T',  'U',  'V',  'W',  'X',  'Y',  'Z',  'Ä',  'Ö',  'Ñ',  'Ü',  '§', // 5
    '¿',  'a',  'b',  'c',  'd',  'e',  'f',  'g',  'h',  'i',  'j',  'k',  'l',  'm',  'n',  'o', // 6
    'p',  'q',  'r',  's',  't',  'u',  'v',  'w',  'x',  'y',  'z',  'ä',  'ö',  'ñ',  'ü',  'à'  // 7
    
];

fn parse_gsm_alphabet(pdu_string: &[u8], length: u8) -> IResult<&[u8], String> {
    let mut parsed_octets = 0;
    let mut output = String::new();
    let mut rest = pdu_string;

    let mut saved_byte: u8 = 0;
    while parsed_octets < length {
        let parse_stage = parsed_octets % 8;
        if parse_stage == 7 {
            output.push(GSM_CHARS[saved_byte as usize]);
            saved_byte = 0;
            parsed_octets += 1;
            continue;
        }

        let (new_rest, next_byte) = hex_octet(rest).unwrap();
        rest = new_rest;
        let character = (next_byte & GSM_MASKS[parse_stage as usize]) << parse_stage;
        
        output.push(GSM_CHARS[(character + saved_byte) as usize]);
        saved_byte = (next_byte & !GSM_MASKS[parse_stage as usize]) >> (7 - parse_stage);
        parsed_octets += 1;
    };

    IResult::Done(rest, output)
}

named!(u8_vec_to_u16_vec < &[u8], Vec<u16> >, many0!(
    map_res!(take!(4), u16_from_hex_str)));

fn parse_utf16(data: &[u8], length: usize) -> IResult<&[u8], String> {
    // length is in 16-bit groups, so we need to double it to get the full string
    let u16_len = length*2;
    if data.len() < u16_len {
        return IResult::Incomplete(nom::Needed::Size(u16_len - data.len()))
    }
    
    let utf16_str: Vec<u16> = u8_vec_to_u16_vec(&data[..u16_len]).to_result().unwrap();
    match String::from_utf16(utf16_str.as_ref()) {
        Ok(s) => IResult::Done(&data[u16_len..], s),
        Err(_) => IResult::Error(nom::ErrorKind::Custom(0))
    }
}
