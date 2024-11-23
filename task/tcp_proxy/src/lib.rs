#![forbid(unsafe_code)]

use std::io::copy;
use std::net::{TcpListener, TcpStream};
use std::thread;

pub fn run_proxy(port: u32, destination: String) {
    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(address).unwrap();

    for incoming_stream in listener.incoming() {
        match incoming_stream {
            Ok(stream) => {
                let destination = destination.clone();

                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, &destination) {
                        log::error!("error handling connection: {err}");
                    }
                });
            }
            Err(err) => log::error!("failed to accept connection: {err}"),
        }
    }
}

fn handle_connection(mut source: TcpStream, destination_address: &str) -> std::io::Result<()> {
    log::info!(
        "proxying traffic: {} <-> {}",
        source.peer_addr()?,
        destination_address
    );

    let mut destination = TcpStream::connect(destination_address)?;

    let mut source_clone = source.try_clone()?;
    let mut destination_clone = destination.try_clone()?;

    let source_to_destination = thread::spawn(move || copy(&mut source, &mut destination));
    let destination_to_source =
        thread::spawn(move || copy(&mut destination_clone, &mut source_clone));

    source_to_destination.join().unwrap()?;
    destination_to_source.join().unwrap()?;

    Ok(())
}
