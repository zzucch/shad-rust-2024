use anyhow::Result;

use std::{
    any::Any,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

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

#[cfg(unix)]
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
    pub result: Result<()>,
}

pub struct WasmStrategyRunner {
    engine: Engine,
    path: PathBuf,
    stdin: Option<Box<dyn WasiFile>>,
    stdout: Option<Box<dyn WasiFile>>,
    stderr: Option<Box<dyn WasiFile>>,
    cpu_fuel_limit: u64,
    memory_size_limit: usize,
}

impl WasmStrategyRunner {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);

        Self {
            engine: Engine::new(&config).expect("engine config is invalid"),
            path: path.into(),
            stdin: None,
            stdout: None,
            stderr: None,
            cpu_fuel_limit: u64::MAX,
            memory_size_limit: usize::MAX,
        }
    }

    pub fn stdin(mut self, stdin: impl IntoWasiFile) -> Self {
        self.stdin = Some(Box::new(stdin.into_wasi_file()));
        self
    }

    pub fn stdout(mut self, stdout: impl IntoWasiFile) -> Self {
        self.stdout = Some(Box::new(stdout.into_wasi_file()));
        self
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

    pub fn make_iterrupter(&self) -> Interrupter {
        Interrupter {
            engine: self.engine.clone(),
        }
    }

    pub fn run(self) -> Result<RunStatus> {
        struct AppState {
            wasi_ctx: WasiCtx,
            store_limits: StoreLimits,
        }

        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s: &mut AppState| &mut s.wasi_ctx)?;

        let mut wasi_ctx_builder = WasiCtxBuilder::new();
        if let Some(stdin) = self.stdin {
            wasi_ctx_builder = wasi_ctx_builder.stdin(stdin);
        }
        if let Some(stdout) = self.stdout {
            wasi_ctx_builder = wasi_ctx_builder.stdout(stdout);
        }
        if let Some(stderr) = self.stderr {
            wasi_ctx_builder = wasi_ctx_builder.stderr(stderr);
        }
        let wasi_ctx = wasi_ctx_builder.build();

        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_size_limit)
            .trap_on_grow_failure(true)
            .build();

        let mut store = Store::new(
            &self.engine,
            AppState {
                wasi_ctx,
                store_limits,
            },
        );
        store.add_fuel(self.cpu_fuel_limit)?;
        store.limiter(|s| &mut s.store_limits);
        store.set_epoch_deadline(1);

        let module = Module::from_file(&self.engine, self.path)
            .map_err(|e| e.context("failed to load wasm file"))?;
        linker.module(&mut store, "strategy", &module)?;

        let result = linker
            .get_default(&mut store, "strategy")?
            .typed::<(), ()>(&store)?
            .call(&mut store, ());

        Ok(RunStatus {
            fuel_consumed: store.fuel_consumed().unwrap(),
            result,
        })
    }
}

pub struct Interrupter {
    engine: Engine,
}

impl Interrupter {
    pub fn interrupt(self) {
        self.engine.increment_epoch();
    }
}
