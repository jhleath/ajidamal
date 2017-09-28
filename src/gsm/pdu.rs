use std::str;
use std::num::ParseIntError;
use nom::IResult;

#[derive(Debug)]
pub struct Message {
    service_center: String,
    command_type: u8,
    sender: String,
    time_stamp: String,
    protocol_id: u8,
    encoding_scheme: u8,
    user_data: Vec<u8>,
    // Message requires that user data is valid UTF-8.
}

enum Error {
    ParseError
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
    Ok((data / 2) + 1)
}

fn to_vec(data: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(data.to_vec())
}

named!(hex_octet<u8>, map_res!(take!(2), u8_from_hex_str));

named!(decimal_octet<String>, map_res!(take!(2), str_from_decimal_octet));

named_args!(decimal_octet_number(length: u8)<String>,
            map_res!(
                count!(decimal_octet, length as usize),
                concat_strings));

named!(parse_pdu<Message>,
       do_parse!(
           sc_length: hex_octet >>
           sc_address_type: hex_octet >>
           service_center: apply!(decimal_octet_number, sc_length - 1) >>
           message_type: hex_octet >>
           sender_length: map_res!(hex_octet, get_decimal_length) >>
           sender_addres_type: hex_octet >>    
           sender: apply!(decimal_octet_number, sender_length) >>
           protocol_id: hex_octet >>
           encoding_scheme: hex_octet >>
           time_stamp: apply!(decimal_octet_number, 7) >>
           ud_length: hex_octet >>
           user_data: map_res!(take!(ud_length), to_vec) >>
               
           (Message {
               service_center: service_center,
               command_type: message_type,
               sender: sender,
               protocol_id: protocol_id,
               encoding_scheme: encoding_scheme,
               time_stamp: time_stamp,
               user_data: user_data
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

const gsm_masks: &[u8] = &[
    0b01111111, // 1
    0b00111111, // 2
    0b00011111, // 3
    0b00001111, // 4
    0b00000111, // 5
    0b00000011, // 6
    0b00000001, // 7
    0b11111111, // 0
];

const gsm_chars: &[char] = &[
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

fn parse_gsm_alphabet(pdu_string: &[u8]) -> Result<String, ()> {
    let mut parsed_octets = 0;
    let mut output = String::new();
    let mut rest = pdu_string;
    let length = pdu_string.len();

    let mut saved_byte: u8 = 0;
    while parsed_octets < length {
        let parse_stage = parsed_octets % 8;
        if parse_stage == 7 {
            output.push(gsm_chars[saved_byte as usize]);
            saved_byte = 0;
            parsed_octets += 1;
            continue;
        }

        let (new_rest, next_byte) = hex_octet(rest).unwrap();
        rest = new_rest;
        let character = (next_byte & gsm_masks[parse_stage as usize]) << parse_stage;
        
        output.push(gsm_chars[(character + saved_byte) as usize]);
        saved_byte = (next_byte & !gsm_masks[parse_stage as usize]) >> (7 - parse_stage);
        parsed_octets += 1;
    };

    Ok(output)
}


fn combine_u8s(data: &[u8]) -> Result<u16, ()> {
    Ok(((data[0] as u16) << 8) + (data[1] as u16))
}

named!(u8_vec_to_u16_vec < &[u8], Vec<u16> >, many0!(
    map_res!(take!(2), combine_u8s)));

fn parse_utf16(pdu_string: &[u8]) -> Result<String, ()> {
    let utf16_str: Vec<u16> = u8_vec_to_u16_vec(pdu_string).to_result().unwrap();
    match String::from_utf16(utf16_str.as_ref()) {
        Ok(s) => Ok(s),
        Err(_) => Err(()),
    }
}
