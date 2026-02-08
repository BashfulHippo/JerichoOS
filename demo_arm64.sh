#!/bin/bash
# JerichoOS ARM64 Demo Runner
#
# Single-command demo execution with clean output extraction

set -euo pipefail

echo "========================================"
echo "========================================"
echo "========================================"
echo ""

# Build kernel
echo "* Building ARM64 kernel..."
./build_arm64.sh 2>&1 | grep -F -e "Building" -e "✓" -e "*" | tail -5 || true
echo "* Build complete"
echo ""

# Run demos with timeout
echo "> Running demo suite (15 second timeout)..."
echo ""

# Use QEMU's built-in file serial backend instead of shell I/O redirection.
# This avoids WSL2-specific issues where -serial stdio output vanishes when
# stdout is redirected through a subshell (PTY vs pipe chardev mismatch in
# QEMU 6.2.0's stdio backend). With -serial file:, QEMU opens and writes
# the file directly - no shell pipes, no fd inheritance, no buffering issues.
rm -f /tmp/arm64_demo_raw.txt
timeout 15s qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -m 512M \
    -kernel target/aarch64/kernel_arm64.bin \
    -serial file:/tmp/arm64_demo_raw.txt \
    -display none 2>/dev/null || true
sync  # Force filesystem sync

# Diagnostic: verify output was captured
if [ ! -s /tmp/arm64_demo_raw.txt ]; then
    echo "WARNING: QEMU serial output file is empty."
    echo "  Trying fallback: -serial stdio with script(1) for PTY emulation..."
    rm -f /tmp/arm64_demo_raw.txt
    script -q -e -c "timeout 15s qemu-system-aarch64 \
        -machine virt -cpu cortex-a57 -m 512M \
        -kernel target/aarch64/kernel_arm64.bin \
        -serial stdio -display none" /tmp/arm64_demo_raw.txt </dev/null 2>/dev/null || true
    sync
fi

# Extract text from binary output
DEMO_OUTPUT=$(strings /tmp/arm64_demo_raw.txt)
NORMALIZED_OUTPUT=${DEMO_OUTPUT//$'\n'/ }

# Extract and display demo results
echo "========================================"
echo "========================================"
echo "========================================"
echo ""

# Check each demo
failed=0
suite_ok=0
for i in 1 2 3 4 5; do
    if grep -Eq "\\[DEMO[[:space:]]+$i\\].*COMPLETE" <<<"$NORMALIZED_OUTPUT"; then
        DEMO_NAME=$(grep -m1 "\\[DEMO $i\\]" <<<"$DEMO_OUTPUT" | sed -E 's/.*\[DEMO [0-9]+\][[:space:]]*//' | sed -E 's/[[:space:]]*\(.*$//')
        if [ -z "$DEMO_NAME" ]; then
            DEMO_NAME="Detected"
        fi
        echo "* Demo $i: $DEMO_NAME"
        echo "DEMO_RESULT:$i:PASS"
    else
        echo "x Demo $i: FAILED or INCOMPLETE"
        echo "DEMO_RESULT:$i:FAIL"
        failed=1
    fi
done

echo ""

# Extract key validation points
echo "========================================"
echo "========================================"
echo "========================================"
echo ""

# Demo 4: MQTT message delivery
if grep -q "Delivered.*messages to subscriber" <<<"$DEMO_OUTPUT"; then
    MSG_COUNT=$(grep -o "Delivered [0-9]* messages" <<<"$DEMO_OUTPUT" | head -1 | grep -o "[0-9]*")
    echo "* MQTT Delivery: $MSG_COUNT messages delivered"
else
    echo "!  MQTT Delivery: Not detected"
fi

# Demo 5: Capability enforcement
if grep -q "IPC-DENIED" <<<"$DEMO_OUTPUT"; then
    echo "* Security: IPC denied (capability enforcement working)"
else
    echo "!  Security: IPC enforcement not detected"
fi

# Completion marker
if grep -q "All WASM Demos Complete" <<<"$DEMO_OUTPUT"; then
    echo "Suite: All demos completed successfully"
    suite_ok=1
else
    echo "Suite: Incomplete execution"
fi

echo ""

# Performance summary
echo "========================================"
echo "========================================"
echo "========================================"
echo ""

# Note: ARM64 serial output has limited formatting, numeric values may not display
echo "!  Note: ARM64 UART has limited format support (numeric values may show as {})"
echo "- Platform: ARM64 (AArch64)"
echo "- Heap: 8 MB"
echo "🖥️  Machine: QEMU virt (cortex-a57)"
echo ""

# Save processed output
echo "$DEMO_OUTPUT" > /tmp/jericho_arm64_demo.txt
echo "- Full output saved to: /tmp/jericho_arm64_demo.txt"
echo ""

if [ "$failed" -eq 0 ] && [ "$suite_ok" -eq 1 ]; then
    echo "RESULT: PASS"
else
    echo "RESULT: FAIL"
fi

echo "* Demo run complete!"

if [ "$failed" -ne 0 ] || [ "$suite_ok" -ne 1 ]; then
    exit 1
fi
