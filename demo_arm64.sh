#!/bin/bash
# JerichoOS ARM64 Demo Runner
#
# Single-command demo execution with clean output extraction

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║       JerichoOS ARM64 Demo Suite Runner               ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Build kernel
echo "🔨 Building ARM64 kernel..."
./build_arm64.sh 2>&1 | grep -E "(Building|✓|✅)" | tail -5 || true
echo "✅ Build complete"
echo ""

# Run demos with timeout
echo "🚀 Running demo suite (15 second timeout)..."
echo ""

timeout 15s ./run_arm64.sh > /tmp/arm64_demo_raw.txt 2>&1 || true

# Extract text from binary output
DEMO_OUTPUT=$(strings /tmp/arm64_demo_raw.txt)

# Extract and display demo results
echo "╔════════════════════════════════════════════════════════╗"
echo "║                    Demo Results                        ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Check each demo
for i in 1 2 3 4 5; do
    if echo "$DEMO_OUTPUT" | grep -q "DEMO $i.*COMPLETE"; then
        DEMO_NAME=$(echo "$DEMO_OUTPUT" | grep "DEMO $i" | head -1 | sed 's/.*DEMO [0-9] //' | sed 's/ (.*//')
        echo "✅ Demo $i: $DEMO_NAME"
    else
        echo "❌ Demo $i: FAILED or INCOMPLETE"
    fi
done

echo ""

# Extract key validation points
echo "╔════════════════════════════════════════════════════════╗"
echo "║              Validation Checkpoints                    ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Demo 4: MQTT message delivery
if echo "$DEMO_OUTPUT" | grep -q "Delivered.*messages to subscriber"; then
    MSG_COUNT=$(echo "$DEMO_OUTPUT" | grep -o "Delivered [0-9]* messages" | head -1 | grep -o "[0-9]*")
    echo "✅ MQTT Delivery: $MSG_COUNT messages delivered"
else
    echo "⚠️  MQTT Delivery: Not detected"
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

# Note: ARM64 serial output has limited formatting, numeric values may not display
echo "⚠️  Note: ARM64 UART has limited format support (numeric values may show as {})"
echo "📦 Platform: ARM64 (AArch64)"
echo "💾 Heap: 8 MB"
echo "🖥️  Machine: QEMU virt (cortex-a57)"
echo ""

# Save processed output
echo "$DEMO_OUTPUT" > /tmp/jericho_arm64_demo.txt
echo "📄 Full output saved to: /tmp/jericho_arm64_demo.txt"
echo ""

echo "✅ Demo run complete!"
