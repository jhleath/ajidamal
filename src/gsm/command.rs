use std::sync::mpsc;

// Phone Modes:
// PDU(0) vs Text mode(1) for SMS = AT+CMGF

type RawCallback = mpsc::Sender<(CommandType, String)>;

#[derive(PartialEq, Debug)]
pub enum CommandType {
    Attention, // AT
    Hangup, // ATH
    Dial, // ATD
    SignalQuality, // AT+CSQ
    OperatorSelect, // AT+COPS
    NetworkSystemMode,
    ReadSMS,
    ListSMS,
    SendSMS,
    GetSMSC
}

type CommandIssueResult = Result<(), mpsc::SendError<RawCommand>>;

pub struct RawCommand {
    bytes: Vec<u8>,
    write_cr: bool,
    sender: Option<RawCallback>,
    command_type: CommandType,
}

impl RawCommand {
    pub fn get_callback(self) -> Option<(CommandType, RawCallback)> {
        match self.sender {
            Some(s) => Some((self.command_type, s)),
            None => None
        }
    }

    pub fn render(&self) -> Vec<u8> {
        let mut output = self.bytes.clone();

        if self.write_cr {
            output.push(b'\r');
        }

        output
    }
}

pub struct Pipeline {
    phone: mpsc::Sender<RawCommand>,
}

impl Pipeline {
    pub fn new(phone: mpsc::Sender<RawCommand>) -> Pipeline {
        Pipeline {
            phone: phone,
        }
    }

    fn send_command(&self, cmd: RawCommand) -> CommandIssueResult {
        self.phone.send(cmd)
    }

    pub fn attention(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: "AT".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::Attention,
        })
    }

    pub fn hangup(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        // VOICE CALL: END:
        self.send_command(RawCommand {
            bytes: "ATH".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::Hangup,
        })
    }

    pub fn dial(&self, number: &str, sender: Option<RawCallback>) -> CommandIssueResult {
        // VOICE CALL: BEGIN:
        self.send_command(RawCommand {
            bytes: format!("ATD{};", number).as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::Dial,
        })
    }

    pub fn signal_quality(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: "AT+CSQ?".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::SignalQuality,
        })
    }

    pub fn operator_select(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: "AT+COPS?".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::OperatorSelect,
        })
    }

    pub fn network_system_mode(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: "AT+CNSMOD?".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::NetworkSystemMode
        })
    }

    pub fn read_sms(&self, index: u32, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: format!("AT+CMGR={}", index).as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::ReadSMS,
        })
    }

    pub fn list_sms(&self, store: SMSStore, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: format!("AT+CMGL={}", store as i32).as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::ListSMS,
        })
    }

    pub fn send_sms(&self, data: Vec<u8>, sender: Option<RawCallback>) -> CommandIssueResult {
        let string_command = String::from_utf8(data).unwrap();
        println!("sending sms command {}", string_command);

        self.send_command(RawCommand {
            bytes: format!("AT+CMGS={}", string_command.len() / 2).as_bytes().to_vec(),
            write_cr: true,
            sender: None,
            command_type: CommandType::SendSMS,
        }).unwrap();

        self.send_command(RawCommand {
            bytes: format!("{}\u{001a}", string_command).as_bytes().to_vec(),
            write_cr: false,
            sender: sender,
            command_type: CommandType::SendSMS,
        })
    }

    pub fn get_smsc(&self, sender: Option<RawCallback>) -> CommandIssueResult {
        self.send_command(RawCommand {
            bytes: "AT+CSCA?".as_bytes().to_vec(),
            write_cr: true,
            sender: sender,
            command_type: CommandType::GetSMSC,
        })
    }
    // Ringing: 2
    // MISSED_CALL: 09:21AM <NUM>
}

pub enum SMSStore {
    All = 4
}
