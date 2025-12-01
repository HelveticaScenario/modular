#!/bin/bash
# Quick demo script for audio streaming oscilloscope

echo "🎛️  Modular Audio Streaming Demo"
echo "================================"
echo ""

# Check if server is running
if ! curl -s http://localhost:7812/health > /dev/null 2>&1; then
    echo "❌ Server not running. Please start it first:"
    echo "   cargo run --bin modular_server -- --port 7812"
    echo ""
    exit 1
fi

echo "✓ Server is running"
echo ""

# Create a sine wave oscillator
echo "Creating 440Hz sine wave oscillator..."
curl -X POST http://localhost:7812/modules \
  -H "Content-Type: application/json" \
  -d '{"module_type":"sine-oscillator","id":"sine-1"}' 2>/dev/null

sleep 0.2

# Set frequency (v/oct, 4.0v = 440Hz)
echo "Setting frequency to 440Hz (4.0v)..."
curl -X PUT "http://localhost:7812/params/sine-1/freq" \
  -H "Content-Type: application/json" \
  -d '{"param":{"param_type":"value","value":4.0}}' 2>/dev/null

sleep 0.2

# Connect to root
echo "Connecting to audio output..."
curl -X PUT "http://localhost:7812/params/root/source" \
  -H "Content-Type: application/json" \
  -d '{"param":{"param_type":"cable","module":"sine-1","port":"output"}}' 2>/dev/null

echo ""
echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Open oscilloscope.html in your browser"
echo "2. Click 'Connect'"
echo "3. Module ID: sine-1"
echo "4. Port: output"
echo "5. Click 'Subscribe to Audio'"
echo ""
echo "You should now see a 440Hz sine wave!"
