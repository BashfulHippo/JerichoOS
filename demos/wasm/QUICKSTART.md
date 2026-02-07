# WASM Demos Quick Start

## 1. Install WABT (optional, for rebuilding `.wat` demos)

```bash
sudo apt-get install wabt
```

## 2. Rebuild `.wat` demos

```bash
cd demos/wasm
make
```

## 3. Run kernel demo suite

x86-64:

```bash
cd ../..
./demo_x86.sh
```

AArch64:

```bash
cd ../..
./demo_arm64.sh
```

## Validation

- x86-64 script prints `DEMO_RESULT:<n>:PASS` markers
- AArch64 output includes `DEMO <n> ... COMPLETE` markers
