use std::io::{self, BufRead, Write};

use paperio_proto::{
    traits::{ProtoRead, ProtoWrite},
    CommandMessage, GameParams, Message, World,
};

pub(crate) struct PlayerEndpoint<'a> {
    r: Box<dyn BufRead + 'a>,
    w: Box<dyn Write + 'a>,
}

impl<'a> PlayerEndpoint<'a> {
    pub fn new(r: impl BufRead + 'a, w: impl Write + 'a) -> Self {
        PlayerEndpoint {
            r: Box::new(r),
            w: Box::new(w),
        }
    }

    pub fn send_message(&mut self, msg: &Message) -> io::Result<()> {
        msg.write(&mut self.w)?;
        self.w.flush()
    }

    pub fn send_start_game(&mut self, params: GameParams) -> io::Result<()> {
        self.send_message(&Message::StartGame(params))
    }

    pub fn send_tick(&mut self, world: World) -> io::Result<()> {
        self.send_message(&Message::Tick(world))
    }

    pub fn send_end_game(&mut self) -> io::Result<()> {
        self.send_message(&Message::EndGame {})
    }

    pub fn get_tick(&mut self) -> io::Result<CommandMessage> {
        CommandMessage::read(&mut self.r)
    }
}
