#!/bin/bash
# JerichoOS x86-64 Demo Runner
#
# Single-command demo execution with clean output extraction
# WSL-compatible version using file-based serial output

set -euo pipefail

echo "========================================"
echo "========================================"
echo "========================================"
echo ""

# Build kernel
echo "* Building x86-64 kernel..."
cargo build --bin jericho_os --release 2>&1 | grep -E "(Compiling|Finished)" | tail -5 || true
echo "* Build complete"
echo ""

# Find boot image path
BOOT_IMAGE=$(find target/x86_64-unknown-none/release/build -name "boot-bios.img" 2>/dev/null | head -1)

if [ -z "$BOOT_IMAGE" ]; then
    echo "x Boot image not found!"
    exit 1
fi

# Run demos with timeout - use file-based serial output (WSL-compatible)
echo "> Running demo suite (15 second timeout)..."
echo ""

# Clear previous output
rm -f /tmp/jericho_raw_output.txt

# Run QEMU with serial output to file (more reliable in WSL than stdio)
timeout 15s qemu-system-x86_64 \
    -drive format=raw,file="$BOOT_IMAGE" \
    -serial file:/tmp/jericho_raw_output.txt \
    -display none \
    2>/dev/null || true

# Read and filter the output
if [ -f /tmp/jericho_raw_output.txt ] && [ -s /tmp/jericho_raw_output.txt ]; then
    DEMO_OUTPUT=$(strings /tmp/jericho_raw_output.txt)
    NORMALIZED_OUTPUT=${DEMO_OUTPUT//$'\n'/ }
else
    echo "x No output captured from QEMU"
    exit 1
fi

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

# Demo 4: MQTT pub/sub validation
if grep -q "Full pub/sub flow working" <<<"$DEMO_OUTPUT"; then
    echo "* MQTT: Full pub/sub flow validated (broker + publisher + subscriber)"
elif grep -Eq "\\[DEMO[[:space:]]+4\\].*COMPLETE" <<<"$NORMALIZED_OUTPUT"; then
    echo "* MQTT: Demo 4 complete"
else
    echo "!  MQTT: Not detected"
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

if grep -q "Boot time:" <<<"$DEMO_OUTPUT"; then
    BOOT_TIME=$(grep "Boot time:" <<<"$DEMO_OUTPUT" | head -1 | sed 's/.*Boot time: //' | sed 's/ .*//')
    echo "-  Boot Time: $BOOT_TIME"
fi

echo "- Platform: x86-64 (UEFI)"
echo "- Heap: 8 MB"
echo ""

# Save processed output
echo "$DEMO_OUTPUT" > /tmp/jericho_x86_demo.txt
echo "- Processed output saved to: /tmp/jericho_x86_demo.txt"
echo "- Raw output saved to: /tmp/jericho_raw_output.txt"
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
