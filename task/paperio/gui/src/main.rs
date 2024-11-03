use std::{
    future::Future,
    io::{BufReader, BufWriter},
    net::TcpStream,
    thread,
};

use clap::Parser;
use paperio_gui::app::PaperioApp;

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

    stderrlog::new()
        .verbosity(log::Level::Debug)
        .module(module_path!())
        .init()
        .expect("failed to initialize stderr logger");

    let stream = TcpStream::connect(format!("{}:{}", args.address, args.port))
        .expect("failed to connect to tcp socket");
    let stream_clone = stream.try_clone().expect("failed to clone tcp stream");

    // run gui in current thread
    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|b| b.with_inner_size((1200., 980.)))),
        ..Default::default()
    };
    let app = PaperioApp::new(args.tick_delay_ms, args.spectator);
    let reader = BufReader::new(stream);
    let writer = BufWriter::new(stream_clone);
    let mut backend_future = Box::pin(app.run_backend(reader, writer));
    let handle = thread::spawn(move || {
        let waker = futures::task::noop_waker();
        let mut ctx = futures::task::Context::from_waker(&waker);
        match backend_future.as_mut().poll(&mut ctx) {
            std::task::Poll::Ready(result) => {
                log::info!("result: {result:?}");
                result
            }
            std::task::Poll::Pending => unreachable!(),
        }
    });
    eframe::run_native("paperio", native_options, Box::new(|_| Ok(Box::new(app)))).unwrap();
    handle.join().unwrap().unwrap();
}
