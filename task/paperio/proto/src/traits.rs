use std::io::{self, BufRead, Write};

use crate::{Command, Message};

pub trait JsonRead {
    fn read_message(&mut self) -> io::Result<Message>;
    fn read_command(&mut self) -> io::Result<Command>;
}

pub trait JsonWrite {
    fn write_message(&mut self, message: &Message) -> io::Result<()>;
    fn write_command(&mut self, command: &Command) -> io::Result<()>;
}

impl<T: BufRead> JsonRead for T {
    fn read_message(&mut self) -> io::Result<Message> {
        let mut line = String::new();
        self.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(|err| err.into())
    }

    fn read_command(&mut self) -> io::Result<Command> {
        let mut line = String::new();
        self.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(|err| err.into())
    }
}

impl<T: Write> JsonWrite for T {
    fn write_message(&mut self, message: &Message) -> io::Result<()> {
        serde_json::to_writer(&mut *self, &message)?;
        self.write_all(b"\n")
    }

    fn write_command(&mut self, command: &Command) -> io::Result<()> {
        serde_json::to_writer(&mut *self, &command)?;
        self.write_all(b"\n")
    }
}
