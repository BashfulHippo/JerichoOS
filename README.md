# JerichoOS

JerichoOS is a Rust `no_std` microkernel experiment with a capability model and a WebAssembly runtime (`wasmi`) for untrusted module execution.

## Scope

This project is focused on:
- capability-based access control for kernel services,
- a minimal scheduler/IPC core,
- running WASM modules in a bare-metal context,
- keeping x86-64 and AArch64 ports in one codebase.

This is a research/learning kernel, not a production operating system.

## Current Status

| Area | x86-64 | AArch64 |
|---|---|---|
| Kernel builds | Yes | Yes |
| Boots in QEMU | Yes | Yes |
| WASM demo suite | Yes | Yes |
| CI workflow | Yes | Yes |

Verified locally on February 7, 2026:
- `cargo check --bin jericho_os --release`
- `cargo check --bin jericho_os_arm64 --release --target aarch64-jericho.json -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem`

## Architecture Snapshot

```text
WASM modules (sandboxed)
  -> wasmi host bridge
  -> capability checks
  -> syscall / IPC interfaces
  -> scheduler + memory + interrupt handling
  -> architecture layer (x86-64 or AArch64)
```

## Quick Start

Prerequisites:
- Rust nightly with `rust-src` and `llvm-tools-preview`
- QEMU (`qemu-system-x86`, `qemu-system-aarch64`)
- WABT (`wat2wasm`) only if you want to rebuild `.wat` demos

### x86-64 demo run

```bash
./demo_x86.sh
```

### AArch64 demo run

```bash
./build_arm64.sh
./run_arm64.sh
```

## Benchmark Notes

See `BENCHMARKS.md`.

Important caveat: these numbers are from QEMU-based runs and should be treated as comparative kernel-internal signals, not hardware-accurate production benchmarks.

## Repo Layout

- `src/`: kernel implementation
- `src/arch/aarch64/`: AArch64-specific architecture code
- `demos/wasm/`: WASM demo inputs/artifacts used by kernel demo suite
- `demo_x86.sh`, `demo_arm64.sh`: demo runners
- `bench_x86.sh`, `bench_arm64.sh`: benchmark runners
- `.github/workflows/`: CI pipelines for x86-64 and AArch64
- `docs/PROJECT_STATUS.md`: concise status and known limitations
- `DECISIONS.md`: architectural decision records

## Reproducibility Notes

- `01_add.wasm`, `02_hello.wasm`, and `03_syscall.wasm` are generated from `.wat` sources in `demos/wasm/`.
- MQTT and security demo modules are currently vendored as prebuilt `.wasm` artifacts in `demos/wasm/`.

## Known Limitations

- AArch64 UART formatting is currently limited; some numeric prints are placeholders.
- AArch64 memory setup is still conservative and not a full production-grade virtual memory subsystem.
- The codebase still has warning debt that should be reduced over time.

## Quality Gates

Before pushing:

```bash
cargo check --bin jericho_os --release
cargo check --bin jericho_os_arm64 --release --target aarch64-jericho.json -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem
./demo_x86.sh
```
