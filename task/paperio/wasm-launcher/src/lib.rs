use anyhow::Result;

use std::{
    any::Any,
    io::{Read, Write},
    net::TcpStream,
    os::unix::net::UnixStream,
    path::Path,
};

use wasi_common::{
    file::WasiFile,
    pipe::{ReadPipe, WritePipe},
    WasiCtx,
};
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::WasiCtxBuilder;

pub trait IntoWasiFile {
    fn into_wasi_file(self) -> impl WasiFile + 'static;
}

impl IntoWasiFile for TcpStream {
    fn into_wasi_file(self) -> impl WasiFile {
        wasmtime_wasi::net::TcpStream::from_cap_std(cap_std::net::TcpStream::from_std(self))
    }
}

impl IntoWasiFile for UnixStream {
    fn into_wasi_file(self) -> impl WasiFile {
        wasmtime_wasi::net::UnixStream::from_cap_std(cap_std::os::unix::net::UnixStream::from_std(
            self,
        ))
    }
}

impl<W: Write + Any + Send + Sync> IntoWasiFile for WritePipe<W> {
    fn into_wasi_file(self) -> impl WasiFile + 'static {
        self
    }
}

impl<R: Read + Any + Send + Sync> IntoWasiFile for ReadPipe<R> {
    fn into_wasi_file(self) -> impl WasiFile + 'static {
        self
    }
}

pub struct RunStatus {
    pub fuel_consumed: u64,
    pub fuel_remaining: u64,
    pub result: Result<()>,
}

pub struct WasmStrategyRunner<'a> {
    path: &'a Path,
    stdin: Box<dyn WasiFile>,
    stdout: Box<dyn WasiFile>,
    stderr: Option<Box<dyn WasiFile>>,
    cpu_fuel_limit: u64,
    memory_size_limit: usize,
}

impl<'a> WasmStrategyRunner<'a> {
    pub fn new(
        path: &'a impl AsRef<Path>,
        stdin: impl IntoWasiFile,
        stdout: impl IntoWasiFile,
    ) -> Self {
        Self {
            path: path.as_ref(),
            stdin: Box::new(stdin.into_wasi_file()),
            stdout: Box::new(stdout.into_wasi_file()),
            stderr: None,
            cpu_fuel_limit: u64::MAX,
            memory_size_limit: usize::MAX,
        }
    }

    pub fn stderr(mut self, stderr: impl IntoWasiFile) -> Self {
        self.stderr = Some(Box::new(stderr.into_wasi_file()));
        self
    }

    pub fn cpu_fuel_limit(mut self, limit: u64) -> Self {
        self.cpu_fuel_limit = limit;
        self
    }

    pub fn memory_size_limit(mut self, limit: usize) -> Self {
        self.memory_size_limit = limit;
        self
    }

    pub fn run(self) -> Result<RunStatus> {
        struct AppState {
            wasi_ctx: WasiCtx,
            store_limits: StoreLimits,
        }

        let mut config = Config::new();
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s: &mut AppState| &mut s.wasi_ctx)?;

        let mut wasi_ctx_builder = WasiCtxBuilder::new().stdin(self.stdin).stdout(self.stdout);
        if let Some(stderr) = self.stderr {
            wasi_ctx_builder = wasi_ctx_builder.stderr(stderr);
        }
        let wasi_ctx = wasi_ctx_builder.build();

        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_size_limit)
            .trap_on_grow_failure(true)
            .build();

        let mut store = Store::new(
            &engine,
            AppState {
                wasi_ctx,
                store_limits,
            },
        );
        store.add_fuel(self.cpu_fuel_limit)?;
        store.limiter(|s| &mut s.store_limits);

        let module = Module::from_file(&engine, self.path)
            .map_err(|e| e.context("failed to load wasm file"))?;
        linker.module(&mut store, "strategy", &module)?;

        let result = linker
            .get_default(&mut store, "strategy")?
            .typed::<(), ()>(&store)?
            .call(&mut store, ());

        Ok(RunStatus {
            fuel_consumed: store.fuel_consumed().unwrap(),
            fuel_remaining: store.fuel_remaining().unwrap(),
            result,
        })
    }
}
