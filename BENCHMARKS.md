# Benchmarks

These values are from QEMU-based runs and are best treated as relative indicators for kernel changes, not absolute hardware performance numbers.

## Latest Observed x86-64 Signals

| Metric | Value |
|---|---|
| Syscall latency | ~94 ns |
| IPC throughput | ~11.9M messages/sec |
| Boot time | ~100 ms |

## How to Re-run

```bash
./bench_x86.sh
./bench_arm64.sh
```

## Notes

- ARM64 benchmark execution works, but numeric formatting is currently limited in UART output.
- Always compare results across the same host machine and QEMU version.
