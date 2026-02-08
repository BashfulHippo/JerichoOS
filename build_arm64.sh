#!/bin/bash
# Build JerichoOS for ARM64
#
# This script builds the ARM64 kernel and creates a binary suitable for QEMU

set -euo pipefail

echo "* Building JerichoOS for ARM64"
echo "================================"
echo ""

# Check for required tools (warning only, not fatal)
if ! command -v qemu-system-aarch64 &> /dev/null; then
    echo "!  qemu-system-aarch64 not found (optional for build, required for testing)"
    echo "   Ubuntu/Debian: sudo apt install qemu-system-arm"
fi

# Use dedicated ARM64 target directory
export CARGO_BUILD_TARGET_DIR="target/aarch64"

echo "Building ARM64 kernel..."
# Capture full build output for CI debugging
mkdir -p target/aarch64
if ! cargo --config .cargo/config_aarch64.toml build \
    --bin jericho_os_arm64 \
    --target aarch64-jericho.json \
    --release \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem 2>&1 | tee target/aarch64/build_log.txt; then
    echo "x Build failed - cargo returned non-zero exit code"
    echo "x Build log saved to: target/aarch64/build_log.txt"
    exit 1
fi

# Check if build succeeded
if [ ! -f "target/aarch64/aarch64-jericho/release/jericho_os_arm64" ]; then
    echo "x Build failed - binary not created"
    exit 1
fi

echo "* Kernel built successfully"
echo ""

# Create raw binary
echo "Creating kernel binary..." | tee -a target/aarch64/build_log.txt

# Try rust-objcopy first (if available locally)
if command -v rust-objcopy &> /dev/null; then
    echo "Using rust-objcopy..." | tee -a target/aarch64/build_log.txt
    if ! rust-objcopy \
        --strip-all \
        -O binary \
        target/aarch64/aarch64-jericho/release/jericho_os_arm64 \
        target/aarch64/kernel_arm64.bin 2>&1 | tee -a target/aarch64/build_log.txt; then
        echo "x rust-objcopy failed" | tee -a target/aarch64/build_log.txt
        exit 1
    fi
else
    # Use llvm-objcopy from llvm-tools-preview (installed by CI workflow)
    echo "Using llvm-objcopy from llvm-tools-preview..." | tee -a target/aarch64/build_log.txt

    # Find llvm-objcopy in the llvm-tools lib directory
    SYSROOT=$(rustc --print sysroot)
    OBJCOPY=$(find "$SYSROOT/lib/rustlib" -name llvm-objcopy -type f 2>/dev/null | head -1)

    if [ -z "$OBJCOPY" ] || [ ! -x "$OBJCOPY" ]; then
        echo "x Error: llvm-objcopy not found in rustc sysroot" | tee -a target/aarch64/build_log.txt
        echo "   Searched in: $SYSROOT/lib/rustlib" | tee -a target/aarch64/build_log.txt
        echo "   Please install: rustup component add llvm-tools-preview" | tee -a target/aarch64/build_log.txt
        exit 1
    fi

    echo "Found: $OBJCOPY" | tee -a target/aarch64/build_log.txt
    if ! "$OBJCOPY" \
        --strip-all \
        -O binary \
        target/aarch64/aarch64-jericho/release/jericho_os_arm64 \
        target/aarch64/kernel_arm64.bin 2>&1 | tee -a target/aarch64/build_log.txt; then
        echo "x llvm-objcopy failed" | tee -a target/aarch64/build_log.txt
        exit 1
    fi
fi

SIZE=$(wc -c < target/aarch64/kernel_arm64.bin)
echo "* Binary created: target/aarch64/kernel_arm64.bin ($SIZE bytes)"
echo ""

echo "* ARM64 build complete!"
echo ""
echo "To run in QEMU:"
echo "  ./run_arm64.sh"
echo ""
echo "Or manually:"
echo "  qemu-system-aarch64 -machine virt -cpu cortex-a57 -m 512M \\"
echo "    -kernel target/aarch64/kernel_arm64.bin \\"
echo "    -serial stdio -display none"
