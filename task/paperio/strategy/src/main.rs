#![forbid(unsafe_code)]

use paperio_proto::{
    traits::{ProtoRead, ProtoWrite},
    CommandMessage, Message,
};
use paperio_strategy::strategy::Strategy;

use std::{
    io::{stdin, stdout, BufReader, Read, Write},
    net::TcpStream,
};

fn run(reader: impl Read, mut writer: impl Write) {
    let mut reader = BufReader::new(reader);

    let Ok(Message::StartGame(_)) = Message::read(&mut reader) else {
        panic!("expected the first message to be 'start_game'");
    };

    let mut strategy = Strategy::new();
    while let Ok(Message::Tick(tick_params)) = Message::read(&mut reader) {
        let tick_num = tick_params.tick_num;
        let direction = strategy.on_tick(tick_params);
        let msg = CommandMessage {
            tick_num,
            command: direction,
        };
        msg.write(&mut writer).unwrap();
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
