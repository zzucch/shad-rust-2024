use clap::Parser;
use paperio_wasm_launcher::{execute_wasm, tcp_stream};
use std::net::TcpStream;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    path: String,
    #[arg(short, long, default_value_t = String::from("localhost"))]
    address: String,
    #[arg(short, long, default_value_t = 8000)]
    port: u16,
}

pub fn main() {
    let args = Arguments::parse();

    let stream = TcpStream::connect(format!("{}:{}", args.address, args.port)).unwrap();
    let cloned_stream = stream.try_clone().unwrap();

    execute_wasm(args.path, tcp_stream(stream), tcp_stream(cloned_stream)).unwrap();
}
