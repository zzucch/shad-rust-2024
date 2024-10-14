use std::io::{self, BufRead, Write};

use paperio_proto::{
    traits::{JsonRead, JsonWrite},
    Command, Message,
};

pub trait Endpoint {
    fn send_message(&mut self, message: &Message) -> io::Result<()>;
    fn get_command(&mut self) -> io::Result<Command>;
}

impl<'a, T: Endpoint> Endpoint for &'a mut T {
    fn send_message(&mut self, message: &Message) -> io::Result<()> {
        T::send_message(self, message)
    }

    fn get_command(&mut self) -> io::Result<Command> {
        T::get_command(self)
    }
}

pub struct JsonEndpoint<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> JsonEndpoint<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R: BufRead, W: Write> Endpoint for JsonEndpoint<R, W> {
    fn send_message(&mut self, message: &Message) -> io::Result<()> {
        self.writer.write_message(message)?;
        self.writer.flush()
    }

    fn get_command(&mut self) -> io::Result<Command> {
        self.reader.read_command()
    }
}
