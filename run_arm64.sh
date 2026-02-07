#!/bin/bash
# Run JerichoOS ARM64 kernel in QEMU

set -euo pipefail

# Check if kernel exists
if [ ! -f "target/aarch64/kernel_arm64.bin" ]; then
    echo "x ARM64 kernel not found. Run ./build_arm64.sh first"
    exit 1
fi

echo "🚀 Running JerichoOS ARM64 in QEMU"
echo "==================================="
echo ""

qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -m 512M \
    -kernel target/aarch64/kernel_arm64.bin \
    -serial stdio \
    -display none \
    "$@"
