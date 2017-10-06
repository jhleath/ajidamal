use std::sync::mpsc;

struct Message {
    id: u64,
    sender: String,
    timestamp: u64,
    contents: String
}

pub struct MessagingManager {
    gsm_module: mpsc::Sender<super::command::RawCommand>,
    loaded_messages: Vec<Message>
}

impl MessagingManager {
    pub fn new(gsm_module: mpsc::Sender<super::command::RawCommand>) -> MessagingManager {
        MessagingManager {
            gsm_module: gsm_module,
            loaded_messages: Vec::new(),
        }
    }

    pub fn load_text_messages(&mut self) -> Result<(), ()> {
        // let (send, recv) = mpsc::channel();
        
        // self.gsm_module.send(super::command::Command {
        //     bytes: "AT+CMGL=4".as_bytes().to_vec(),
        //     write_cr: true,
        //     sender: Some(send.clone())
        // });

        // let message_list = recv.recv().unwrap();

        // for line in message_list.lines() {
        //     println!("reading line {}", line);
        // }
        
        Ok(())
    }
}
