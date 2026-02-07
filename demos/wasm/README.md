# WASM Demo Assets

This folder contains WebAssembly modules consumed by the JerichoOS kernel demo suite.

## Source-Backed Modules

These have `.wat` source files in this directory and can be rebuilt:
- `01_add.wasm`
- `02_hello.wasm`
- `03_syscall.wasm`

## Vendored Binary Modules

These are currently kept as prebuilt `.wasm` artifacts:
- `mqtt_broker.wasm`
- `mqtt_publisher.wasm`
- `mqtt_subscriber.wasm`
- `malicious_module.wasm`

## Build `.wat`-Backed Demos

```bash
cd demos/wasm
make
```

or

```bash
cd demos/wasm
./compile.sh
```

Requirements:
- `wat2wasm` from WABT

## How Kernel Uses These

The kernel embeds these modules via `include_bytes!` in `src/demos/wasm_tests.rs`.
