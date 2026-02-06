# wasm demos

these are the test suite for jerichoos. if they pass on x86-64, they should pass on arm64 too.

## what they test

- wasm runtime integration (wasmi)
- host function calls
- capability system
- security isolation
- cross-architecture compatibility

## demos

### 01_add.wat - pure computation

basic wasm execution. tests parameters and recursion.

functions:
- `add(a, b)` - adds two numbers
- `mul(a, b)` - multiplies two numbers
- `factorial(n)` - recursive factorial

expected results:
```
add(2, 3) = 5
mul(7, 6) = 42
factorial(5) = 120
```

### 02_hello.wat - host function calls

tests wasm calling kernel functions.

functions:
- `main()` - prints 42, 100, 255 via host
- `print_range(start, end)` - prints range of numbers

expected results:
```
[WASM] Print called: 42
[WASM] Print called: 100
[WASM] Print called: 255
```

### 03_syscall.wat - capability system

tests syscall bridge and permission checks.

functions:
- `test_syscall()` - valid syscall (should succeed)
- `test_allocate(size)` - memory allocation (needs capability)
- `test_unauthorized()` - unauthorized access (should fail)

expected results:
```
[WASM] Syscall 1 (1, 4096, 10) - SUCCESS
[WASM] Syscall 2 (1024, 0, 0) - SUCCESS (has capability)
[WASM] Syscall 0 (99, 8192, 100) - DENIED (no capability)
```

## building

install wabt (webassembly binary toolkit):
```bash
sudo apt-get install wabt
```

compile wat to wasm:
```bash
chmod +x compile.sh
./compile.sh
```

this generates `.wasm` binaries from `.wat` text files.

## running

the compiled `.wasm` files are embedded in the kernel and executed during boot. see `src/main.rs` for the test harness.

## adding new demos

1. write `.wat` file
2. document purpose and expected results
3. add to `compile.sh`
4. create corresponding test in `src/main.rs`
5. verify on x86-64 first
6. cross-validate on arm64

wasm binary format is extremely compact. typical compression is 90%+.
