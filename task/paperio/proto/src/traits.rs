use std::io::{self, BufRead, Write};

use crate::{CommandMessage, Message};

pub trait ProtoRead: Sized {
    fn read(reader: &mut impl BufRead) -> io::Result<Self>;
}

pub trait ProtoWrite {
    fn write(&self, writer: &mut impl Write) -> io::Result<()>;
}

impl ProtoRead for Message {
    fn read(reader: &mut impl BufRead) -> io::Result<Self> {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(|err| err.into())
    }
}

impl ProtoWrite for Message {
    fn write(&self, mut writer: &mut impl Write) -> io::Result<()> {
        serde_json::to_writer(&mut writer, &self)?;
        writeln!(writer)
    }
}

impl ProtoRead for CommandMessage {
    fn read(reader: &mut impl BufRead) -> io::Result<Self> {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(|err| err.into())
    }
}

impl ProtoWrite for CommandMessage {
    fn write(&self, mut writer: &mut impl Write) -> io::Result<()> {
        serde_json::to_writer(&mut writer, &self)?;
        writeln!(writer)
    }
}
