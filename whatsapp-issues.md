# WhatsApp Integration & Deployment Issues (Post-82629c6)

This document summarizes the attempts and challenges encountered while integrating WhatsApp support and deploying the ClavaMea service on legacy hardware (Intel Core i3-3217U, Ivy Bridge).

## 1. Architectural Changes
- **Direct Integration**: Attempted to move WhatsApp logic from a separate bridge service into the core Rust application using the `whatsapp-rust` library.
- **Dependency Bloat**: This added significant complexity to the build process and introduced conflicts with existing asynchronous runtimes and trait implementations.

## 2. Deployment Challenges (Exit Code 132)
The primary blocker was a `SIGILL` (Illegal Instruction) error occurring on the target server.

### Hardware Constraints
- **Server**: Intel Core i3-3217U (Ivy Bridge, 3rd Gen).
- **Supported Instructions**: SSE4.2, AVX, F16C.
- **Missing Instructions**: AVX2, FMA, BMI1/2.

### Root Causes identified
1. **ONNX Runtime (ort/fastembed)**: The `fastembed` crate relies on `ort` (ONNX Runtime). By default, it downloads pre-compiled binaries optimized for modern CPUs (requiring AVX2).
2. **Library Loading**: Even with `RUSTFLAGS` set to target older CPUs, pre-compiled shared libraries (`.so`) bypass these flags and crash at load/execution time.
3. **Glibc Mismatch**: Building on Debian-based images (`bookworm`) led to linker errors (`__isoc23_strtoll`) because some dependencies were built against newer glibc versions (2.38+) found in Ubuntu 24.04.

## 3. Attempted Workarounds
- **Multi-Arch Build**: Attempted to build for both `amd64` and `arm64` via Docker Buildx. This resulted in extremely slow build times (> 1 hour) due to QEMU emulation.
- **Custom RUSTFLAGS**: Set `target-cpu=x86-64-v2` and `-C target-feature=-avx2,-fma` to force compatibility.
- **Source Compilation**: Forced `ORT_STRATEGY=compile` to build ONNX Runtime from source within the container to respect CPU flags.
- **CMake Flags**: Explicitly disabled AVX2 in ONNX Runtime via `-DONNXRUNTIME_DISABLE_AVX2=ON`.
- **Diagnostic Bypass**: Temporarily disabled RAG and WASM initialization in `main.rs` to isolate the crash source.

## 4. Conclusion & Lessons Learned
- **Hardware Limitations**: Modern AI/Embedding libraries are increasingly dependent on AVX2+. Running these on hardware older than 4th Gen Intel (Haswell) requires extreme build-time customization or switching to more compatible engines like **Candle**.
- **Build Isolation**: Keeping heavy integrations like WhatsApp in a separate microservice (as originally planned) might be better for isolating dependency and hardware compatibility issues.
- **CI/CD Speed**: Multi-platform builds for heavy Rust apps are not viable on standard GitHub runners without native ARM support or optimized cross-compilation.

## 5. Branching
All code changes related to these attempts have been moved to the branch `feat/whatsapp-integration-attempts`. The `main` branch has been reverted to the last known stable state (`82629c6`).
