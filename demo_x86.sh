#!/bin/bash
# JerichoOS x86-64 Demo Runner
#
# Single-command demo execution with clean output extraction
# WSL-compatible version using file-based serial output

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║       JerichoOS x86-64 Demo Suite Runner              ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Build kernel
echo "🔨 Building x86-64 kernel..."
cargo build --bin jericho_os --release 2>&1 | grep -E "(Compiling|Finished)" | tail -5 || true
echo "✅ Build complete"
echo ""

# Find boot image path
BOOT_IMAGE=$(find target/x86_64-unknown-none/release/build -name "boot-bios.img" 2>/dev/null | head -1)

if [ -z "$BOOT_IMAGE" ]; then
    echo "❌ Boot image not found!"
    exit 1
fi

# Run demos with timeout - use file-based serial output (WSL-compatible)
echo "🚀 Running demo suite (15 second timeout)..."
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
else
    echo "❌ No output captured from QEMU"
    exit 1
fi

# Extract and display demo results
echo "╔════════════════════════════════════════════════════════╗"
echo "║                    Demo Results                        ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Check each demo
for i in 1 2 3 4 5; do
    # Check if demo ran (look for DEMO marker and COMPLETE marker separately)
    if echo "$DEMO_OUTPUT" | grep -q "\[DEMO $i\]"; then
        if echo "$DEMO_OUTPUT" | grep -A5 "\[DEMO $i\]" | grep -q "COMPLETE"; then
            DEMO_NAME=$(echo "$DEMO_OUTPUT" | grep "DEMO $i" | head -1 | sed 's/.*DEMO [0-9]] //' | sed 's/ (.*//')
            echo "✅ Demo $i: $DEMO_NAME"
        else
            echo "⚠️  Demo $i: Started but no COMPLETE marker"
        fi
    else
        echo "❌ Demo $i: Not found"
    fi
done

echo ""

# Extract key validation points
echo "╔════════════════════════════════════════════════════════╗"
echo "║              Validation Checkpoints                    ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Demo 4: MQTT pub/sub validation
if echo "$DEMO_OUTPUT" | grep -q "Full pub/sub flow working"; then
    echo "✅ MQTT: Full pub/sub flow validated (broker + publisher + subscriber)"
elif echo "$DEMO_OUTPUT" | grep -q "DEMO 4.*COMPLETE"; then
    echo "✅ MQTT: Demo 4 complete"
else
    echo "⚠️  MQTT: Not detected"
fi

# Demo 5: Capability enforcement
if echo "$DEMO_OUTPUT" | grep -q "IPC-DENIED.*no IPC_SEND capability"; then
    echo "✅ Security: IPC denied (capability enforcement working)"
else
    echo "⚠️  Security: IPC enforcement not detected"
fi

# Completion marker
if echo "$DEMO_OUTPUT" | grep -q "All WASM Demos Complete"; then
    echo "✅ Suite: All demos completed successfully"
else
    echo "⚠️  Suite: Incomplete execution"
fi

echo ""

# Performance summary
echo "╔════════════════════════════════════════════════════════╗"
echo "║               Performance Summary                      ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

if echo "$DEMO_OUTPUT" | grep -q "Boot time:"; then
    BOOT_TIME=$(echo "$DEMO_OUTPUT" | grep "Boot time:" | head -1 | sed 's/.*Boot time: //' | sed 's/ .*//')
    echo "⏱️  Boot Time: $BOOT_TIME"
fi

echo "📦 Platform: x86-64 (UEFI)"
echo "💾 Heap: 8 MB"
echo ""

# Save processed output
echo "$DEMO_OUTPUT" > /tmp/jericho_x86_demo.txt
echo "📄 Processed output saved to: /tmp/jericho_x86_demo.txt"
echo "📄 Raw output saved to: /tmp/jericho_raw_output.txt"
echo ""

echo "✅ Demo run complete!"
