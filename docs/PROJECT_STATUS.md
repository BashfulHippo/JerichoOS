# Project Status

Last updated: February 7, 2026

## Summary

JerichoOS currently builds on x86-64 and AArch64, boots in QEMU on both, and runs the WASM demo suite through shared runtime code.

## What Is Working

- x86-64 kernel boot path and runtime initialization
- AArch64 kernel boot path and runtime initialization
- capability subsystem initialization and checks
- WASM runtime integration (`wasmi`)
- demo runner scripts for both architectures
- CI workflows for both architectures

## What Is Partially Working

- AArch64 benchmark output is limited by current UART formatting path
- some diagnostics are present but not yet standardized across modules

## Known Technical Debt

- warning count is high on both architecture targets
- AArch64 memory/MMU path is intentionally conservative and needs hardening
- several scripts still prioritize convenience over strict reproducibility

## Evidence / Verification

Local checks executed successfully on February 7, 2026:

```bash
cargo check --bin jericho_os --release
cargo check --bin jericho_os_arm64 --release --target aarch64-jericho.json -Z build-std=core,compiler_builtins,alloc -Z build-std-features=compiler-builtins-mem
```

## Near-Term Priorities

1. Reduce warning surface (focus on dead code and formatting placeholders).
2. Improve ARM64 output formatting path to remove placeholder output.
3. Add stronger automated validation around capability denial paths.
4. Continue tightening docs so every claim maps to a reproducible command.
