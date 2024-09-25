#![forbid(unsafe_code)]

use paperio_proto::{
    traits::{JsonRead, JsonWrite},
    Command, Message,
};
use paperio_strategy::strategy::Strategy;

use std::{
    io::{stdin, stdout, BufReader, Read, Write},
    net::TcpStream,
};

fn run(reader: impl Read, mut writer: impl Write) {
    let mut reader = BufReader::new(reader);

    let Ok(Message::StartGame(_)) = reader.read_message() else {
        panic!("expected the first message to be 'start_game'");
    };

    let mut strategy = Strategy::new();
    while let Ok(Message::Tick(tick_params)) = reader.read_message() {
        let direction = strategy.on_tick(tick_params);
        let msg = Command::ChangeDirection(direction);
        writer.write_command(&msg).unwrap();
        writer.flush().unwrap();
    }
}

pub fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if let Some(port_str) = args.get(1) {
        let port = port_str.parse::<u16>().expect("args[1] should be a u16");
        let stream = TcpStream::connect(format!("localhost:{}", port))
            .expect("failed to connect to tcp socket");
        let cloned_stream = stream.try_clone().unwrap();
        run(stream, cloned_stream);
    } else {
        run(stdin(), stdout());
    }
}
