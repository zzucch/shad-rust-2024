use std::path::Path;

use wasi_common::file::WasiFile;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;

pub fn tcp_stream(stream: std::net::TcpStream) -> Box<dyn WasiFile> {
    let stream = cap_std::net::TcpStream::from_std(stream);
    let stream = wasmtime_wasi::net::TcpStream::from_cap_std(stream);
    Box::new(stream)
}

pub fn execute_wasm(
    wasm_file: impl AsRef<Path>,
    stdin: Box<dyn WasiFile>,
    stdout: Box<dyn WasiFile>,
) -> anyhow::Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    let wasi_ctx = WasiCtxBuilder::new().stdin(stdin).stdout(stdout).build();
    let mut store = Store::new(&engine, wasi_ctx);

    let module =
        Module::from_file(&engine, wasm_file).map_err(|e| e.context("filed to load wasm file"))?;
    linker.module(&mut store, "", &module)?;

    linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?
        .call(&mut store, ())
}
