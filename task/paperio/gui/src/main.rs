mod app;
mod colors;
mod state;

use std::{
    io::{BufReader, BufWriter, Write},
    net::TcpStream,
    sync::mpsc,
    thread,
    time::Duration,
};

use app::{AtomicDirection, PaperioApp};
use clap::Parser;
use paperio_proto::{
    traits::{JsonRead, JsonWrite},
    Command, Direction, Message,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    address: String,
    #[arg(short, long, default_value_t = 8000)]
    port: u16,
    #[arg(short, long, default_value_t = 120)]
    tick_delay_ms: u64,
    #[arg(short, long, action)]
    spectator: bool,
}

fn main() {
    let args = Arguments::parse();

    let stream = TcpStream::connect(format!("{}:{}", args.address, args.port))
        .expect("failed to connect to tcp socket");
    let (msg_to_gui, msg_receiver) = mpsc::channel();
    let atomic_direction_store = AtomicDirection::new(Direction::Left);

    // thread for sending and receiving messages
    let handle = spawn_cmd_thread(
        stream,
        msg_to_gui,
        atomic_direction_store.clone(),
        args.tick_delay_ms,
        args.spectator,
    );

    // run gui in current thread
    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|b| b.with_inner_size((1200., 980.)))),
        ..Default::default()
    };
    eframe::run_native(
        "paperio",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(PaperioApp::new(
                cc,
                msg_receiver,
                atomic_direction_store,
            )))
        }),
    )
    .unwrap();

    let _ = handle.join();
}

fn spawn_cmd_thread(
    stream: TcpStream,
    msg_to_gui: mpsc::Sender<Message>,
    atomic_direction_store: AtomicDirection,
    tick_delay_ms: u64,
    is_spectator: bool,
) -> thread::JoinHandle<anyhow::Result<()>> {
    thread::spawn(move || -> anyhow::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = BufWriter::new(stream);

        // receive `GameParams` msg
        if let Ok(msg) = reader.read_message() {
            msg_to_gui.send(msg)?;
        } else {
            panic!("Could not read first message")
        };

        // receive tick msgs
        while let Ok(msg) = reader.read_message() {
            msg_to_gui.send(msg)?;

            thread::sleep(Duration::from_millis(tick_delay_ms));

            let command = if !is_spectator {
                Command::ChangeDirection(atomic_direction_store.load())
            } else {
                Command::NoOp
            };

            writer.write_command(&command)?;
            writer.flush()?;
        }
        Ok(())
    })
}
