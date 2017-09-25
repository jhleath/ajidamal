#[derive(Debug)]
pub struct Message {
    service_center: String,
    command_type: u64,
    sender: String,
    time_stamp: String,
    protocol_id: u64,
    encoding_scheme: u64,
    user_data: String,
    // Message requires that user data is valid UTF-8.
}

impl Message {
    pub fn from_string(pdu_string: String) -> Result<Message, ()> {
        // Parse the string as octects (groups of two hexadecimal characters)
        let (service_center, rest) = match parse_number(pdu_string.as_ref(), false) {
            Some(s) => s,
            None => return Err(())
        };

        let (message_type, rest) = match get_hex_octet(rest) {
            Some(t) => t,
            None => return Err(())
        };
        
        let (sender, rest) = match parse_number(rest, true) {
            Some(s) => s,
            None => return Err(())
        };
        
        let (pid, rest) = match get_hex_octet(rest) {
            Some(t) => t,
            None => return Err(())
        };
        
        let (dcs, rest) = match get_hex_octet(rest) {
            Some(t) => t,
            None => return Err(())
        };

        let (time_stamp, rest) = match get_decimal_octets(rest, 7) {
            Some(t) => t,
            None => return Err(())
        };

        let (mut length_of_user_message, rest) = match get_hex_octet(rest) {
            Some(t) => t,
            None => return Err(())
        };

        let user_data = match parse_gsm_alphabet(rest, length_of_user_message) {
            Some(t) => t,
            None => return Err(())
        };

        Ok(Message{
            service_center: service_center,
            command_type: message_type as u64,
            sender: sender,
            protocol_id: pid as u64,
            encoding_scheme: dcs as u64,
            time_stamp: time_stamp,
            user_data: user_data,
        })
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

fn parse_gsm_alphabet(pdu_string: &str, length: u8) -> Option<String> {
    let mut parsed_octets = 0;
    let mut output = String::new();
    let mut rest = pdu_string;

    let mut saved_byte: u8 = 0;
    while parsed_octets < length {
        let parse_stage = parsed_octets % 8;
        if parse_stage == 7 {
            output.push(gsm_chars[saved_byte as usize]);
            saved_byte = 0;
            parsed_octets += 1;
            continue;
        }

        let (next_byte, new_rest) = get_hex_octet(rest).unwrap();
        rest = new_rest;
        let character = (next_byte & gsm_masks[parse_stage as usize]) << parse_stage;
        
        output.push(gsm_chars[(character + saved_byte) as usize]);
        saved_byte = (next_byte & !gsm_masks[parse_stage as usize]) >> (7 - parse_stage);
        parsed_octets += 1;
    };

    Some(output)
}

fn parse_number(pdu_string: &str, expecting_decimal_length: bool) -> Option<(String, &str)> {
    get_hex_octet(pdu_string).and_then(|(num_sender_bytes, rest)| {
        if num_sender_bytes == 0 {
            return Some((0, rest))
        }
        
        get_hex_octet(rest).and_then(|(address_format, rest)| {
            assert_eq!(address_format, 145);
            Some((num_sender_bytes - 1, rest))
        })
    }).and_then(|(mut num_sender_bytes, rest)| {
        if expecting_decimal_length {
            num_sender_bytes = (num_sender_bytes / 2) + 1;
        }
        
        get_decimal_octets(rest, num_sender_bytes)
    })
}

fn get_hex_octet(pdu_string: &str) -> Option<(u8, &str)> {
    if pdu_string.len() < 2 {
        return None
    }
    
    let (octet, cdr) = pdu_string.split_at(2);
    Some((u8::from_str_radix(octet, 16).expect("Received invalid characters."),
          cdr))
}

fn get_decimal_octets(pdu_string: &str, num_octets: u8) -> Option<(String, &str)> {
    if num_octets == 0 {
        return Some(("".to_string(), pdu_string))
    }
    
    let mut iter = pdu_string.chars();
    let mut output_string = String::new();
    let mut parsed_octets = 0;
    
    while parsed_octets < num_octets {
        let first = iter.next();
        let second = iter.next();
        
        output_string.push(match second {
            Some(digit) => digit,
            None => return None,
        });
        
        output_string.push(match first {
            Some(digit) => {
                if parsed_octets + 1 == num_octets && digit == 'F' {
                    // A final F shows that there were an odd number
                    // of digits.
                    break;
                }
                
                digit
            },
            None => return None,
        });

        parsed_octets += 1
    }

    Some((output_string, iter.as_str()))
}
