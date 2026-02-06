# project status

last updated: jan 2026

## current state

production ready on both arm64 and x86-64. all 5 demos pass on both platforms.

### what works

- boots in ~100-500ms
- 8mb heap, ~5mb binary size
- capability-based security (sel4-style)
- wasm runtime (wasmi)
- preemptive scheduler
- ipc between modules
- mqtt pub/sub demo
- quantitative benchmarks

### platforms

both arm64 and x86-64 have:
- uefi boot
- all 5 wasm demos passing
- github actions ci
- identical feature set

## demos

1. pure computation - add, multiply, factorial
2. host function calls - wasm calling kernel functions
3. syscalls and capabilities - permission checks
4. mqtt pub/sub - broker, publisher, subscriber working
5. security isolation - malicious module blocked

all demos verified on both platforms.

## benchmarks

- syscall latency: 94ns
- ipc throughput: 11.9M msg/sec
- context switch: <1µs
- boot time: ~100ms in qemu

## known issues

heap allocator has fragmentation issues. needs 8mb for a 1mb allocation. linked_list_allocator is simple but not great. could use buddy/slab/tlsf later.

arm64 mmu is identity-mapped only. works fine for now.

## what this proves

you can build a secure microkernel that's 20-40x smaller than docker containers while maintaining capability-based isolation. wasm runtime works on bare metal. both major architectures supported with unified codebase.

## next steps

maybe add better allocator, multi-core support, more network protocols. formal verification would be cool. but it works as-is for the demo.
