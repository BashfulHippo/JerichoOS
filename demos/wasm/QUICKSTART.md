# wasm demos quick start

## install tools

```bash
sudo apt-get install wabt
```

## build demos

```bash
cd demos/wasm
make
```

compiles all `.wat` text files to `.wasm` binaries.

## test on x86-64

```bash
cd ../..
cargo run
```

look for the wasm demo suite section in output. all 5 demos should pass.

## troubleshooting

**"no such file: demos/wasm/01_add.wasm"**
- run `make` in `demos/wasm/` first

**"wat2wasm: command not found"**
- install wabt: `sudo apt-get install wabt`

**"failed to load module"**
- check validity: `wasm-validate 01_add.wasm`
- try recompiling: `make clean && make`
