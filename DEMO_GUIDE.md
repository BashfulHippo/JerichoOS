# Demo Guide

## Run Full Demo Suites

```bash
./demo_x86.sh
./demo_arm64.sh
```

## Expected x86-64 Markers

- `DEMO_RESULT:1:PASS` through `DEMO_RESULT:5:PASS`
- `Suite: All demos completed successfully`
- `RESULT: PASS`

## Expected AArch64 Markers

- `DEMO_RESULT:1:PASS` through `DEMO_RESULT:5:PASS`
- `RESULT: PASS`

## If a Demo Fails

1. Rebuild target (`./build_arm64.sh` for ARM64, or rerun x86 script).
2. Verify required QEMU binary is installed.
3. Inspect captured output files in `/tmp/jericho_*`.
