//! WebAssembly runtime for safe code execution.
//!
//! Provides a sandboxed environment using `wasmtime` and `WASI`.

use anyhow::{Context, Result};
use wasmtime::*;
use wasmtime_wasi::preview1::{add_to_linker_sync, WasiP1Ctx};
use wasmtime_wasi::{pipe::MemoryOutputPipe, WasiCtxBuilder};

/// Represents the state for a single Wasm execution.
struct WasmState {
    wasi: WasiP1Ctx,
}

/// A sandboxed WebAssembly runtime.
pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    /// Create a new WasmRuntime.
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(false);

        let engine = Engine::new(&config).context("Failed to create Wasmtime engine")?;

        Ok(Self { engine })
    }

    /// Execute WebAssembly Text (WAT) code and return its stdout.
    pub fn execute_wat(&self, wat: &str) -> Result<String> {
        let wasm = wat::parse_str(wat).map_err(|e| anyhow::anyhow!("WAT parsing error: {}", e))?;
        self.execute_wasm(&wasm)
    }

    /// Execute compiled WebAssembly bytes and return its stdout.
    pub fn execute_wasm(&self, wasm_bytes: &[u8]) -> Result<String> {
        let mut linker = Linker::new(&self.engine);
        add_to_linker_sync(&mut linker, |s: &mut WasmState| &mut s.wasi)?;

        let stdout = MemoryOutputPipe::new(1024 * 1024); // 1MB buffer

        // Setup WASI with custom stdout capture
        let wasi = WasiCtxBuilder::new()
            .stdout(stdout.clone())
            .inherit_stderr()
            .build_p1();

        let mut store = Store::new(&self.engine, WasmState { wasi });

        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| anyhow::anyhow!("Wasm compilation error: {}", e))?;

        let instance = linker
            .instantiate(&mut store, &module)
            .context("Failed to instantiate Wasm module")?;

        // Try to find the start function (WASI usually uses _start)
        let exports: Vec<String> = instance
            .exports(&mut store)
            .map(|e| e.name().to_string())
            .collect();
        tracing::debug!("Module exports: {:?}", exports);

        let start = instance
            .get_func(&mut store, "_start")
            .or_else(|| instance.get_func(&mut store, "main"))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No entry point found (_start or main). Available exports: {:?}",
                    exports
                )
            })?;

        start
            .call(&mut store, &[], &mut [])
            .context("Wasm execution failed")?;

        // Retrieve captured stdout
        let output_bytes = stdout.contents();
        let output = String::from_utf8_lossy(&output_bytes).to_string();

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_basic_wat() {
        let runtime = WasmRuntime::new().unwrap();
        // A simple WAT that prints "Hello, Wasm!" via WASI
        let wat = r#"
        (module
            (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
            (memory 1)
            (export "memory" (memory 0))
            (data (i32.const 8) "Hello, Wasm!\n")
            (func $main (export "_start")
                (i32.const 0) (i32.const 8) (i32.store)
                (i32.const 4) (i32.const 13) (i32.store)
                (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 24)
                (call $fd_write)
                drop
            )
        )
        "#;
        let result = runtime.execute_wat(wat).unwrap();
        assert_eq!(result.trim(), "Hello, Wasm!");
    }
}
