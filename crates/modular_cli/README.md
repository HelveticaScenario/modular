# modular_cli

CLI tools for performance analysis and benchmarking the modular synthesizer DSP engine.

## Tools

### modular-perf

Analyze performance logs written by the running synthesizer.

```bash
# Follow log in real-time
modular-perf tail

# Show slowest modules
modular-perf top --limit 20

# Query by module type
modular-perf query --module-type plaits
```

### modular-bench

Standalone benchmark harness for profiling DSP code with native tools (samply, Instruments, perf, Tracy).

```bash
# List available patches
modular-bench list

# Run benchmark with a patch
modular-bench run patches/simple_sine.json --frames 1000000

# Run with per-module stats
modular-bench run patches/full_mix.json --frames 480000 --stats

# Quick smoke test all patches
modular-bench smoke
```

## Profiling Workflows

### 1. Criterion Benchmarks (Statistical Analysis)

Run criterion benchmarks for statistically rigorous measurements:

```bash
# Run all benchmarks
cargo bench -p modular_core

# Run specific benchmark
cargo bench -p modular_core -- sine

# Generate HTML report (opens in browser)
cargo bench -p modular_core -- --plotting-backend plotters
```

Results are saved to `target/criterion/` with HTML reports.

### 2. Samply (macOS/Linux Sampling Profiler)

[Samply](https://github.com/mstange/samply) is a command-line sampling profiler that outputs Firefox Profiler format.

```bash
# Install samply
cargo install samply

# Build with profiling profile (symbols preserved)
cargo build --profile profiling -p modular_cli --bin modular-bench

# Record profile
samply record ./target/profiling/modular-bench run crates/modular_cli/patches/plaits_heavy.json --frames 1000000

# This opens Firefox Profiler in your browser with the results
```

### 3. Instruments (macOS)

Use Xcode Instruments for detailed macOS profiling:

```bash
# Build with profiling profile
cargo build --profile profiling -p modular_cli --bin modular-bench

# Open Instruments and use Time Profiler
# Select the binary: ./target/profiling/modular-bench
# Arguments: run crates/modular_cli/patches/full_mix.json --frames 1000000
```

Or from command line:
```bash
xcrun xctrace record --template 'Time Profiler' \
  --output profile.trace \
  --launch -- ./target/profiling/modular-bench run crates/modular_cli/patches/full_mix.json --frames 1000000

# Open result
open profile.trace
```

### 4. Tracy (Real-time Profiler)

[Tracy](https://github.com/wolfpld/tracy) is a real-time profiler with a graphical viewer.

```bash
# Install Tracy (macOS)
brew install tracy

# Build with profile feature enabled
cargo build --profile profiling -p modular_cli --bin modular-bench --features profile

# Start Tracy GUI
tracy &

# Run the benchmark (Tracy will connect automatically)
./target/profiling/modular-bench run crates/modular_cli/patches/full_mix.json --frames 1000000
```

Tracy provides:
- Real-time frame timing visualization
- Flame graphs
- Memory allocation tracking
- Lock contention analysis

### 5. perf (Linux)

```bash
# Build with profiling profile
cargo build --profile profiling -p modular_cli --bin modular-bench

# Record with perf
perf record -g ./target/profiling/modular-bench run crates/modular_cli/patches/full_mix.json --frames 1000000

# View results
perf report
```

## Benchmark Patches

Located in `patches/`:

| Patch | Description | Complexity |
|-------|-------------|------------|
| `simple_sine.json` | Single sine oscillator | Minimal |
| `poly_stack.json` | 16 sine oscillators mixed | Medium |
| `filter_sweep.json` | Saw → LPF with LFO modulation | Medium |
| `plaits_heavy.json` | 4 Plaits instances | Heavy |
| `full_mix.json` | Multi-voice synth with filters | Heavy |

## Creating Custom Patches

Patches are JSON files matching the `PatchGraph` structure:

```json
{
  "modules": [
    {
      "id": "ROOT_OUTPUT",
      "module_type": "signal",
      "params": {
        "source": { "Cable": { "module": "osc-1", "port": "output", "channel": 0 } }
      }
    },
    {
      "id": "osc-1",
      "module_type": "sine",
      "params": {
        "freq": { "Volts": { "value": 0.0 } }
      }
    }
  ],
  "scopes": []
}
```

Signal types:
- `{ "Volts": { "value": 0.0 } }` - Constant voltage
- `{ "Cable": { "module": "id", "port": "output", "channel": 0 } }` - Module connection
- `{ "Poly": [...] }` - Polyphonic array of signals

## Interpreting Results

### Real-time Budget

At 48kHz, you have ~20,833 ns per frame. The benchmark reports:
- **ns/frame**: Average time per audio frame
- **Budget usage**: Percentage of available real-time budget
- Values >100% mean the DSP cannot run in real-time

### Per-module Stats

Use `--stats` to see which modules consume the most time:

```
Per-module timing:
  Module ID            Type         Count    Avg(ns)    Min(ns)    Max(ns)    Total(ns)
  -------------------- ------------ ---------- ---------- ---------- ---------- ------------
  plaits-1             plaits         480000        412        380        892    197760000
  lpf-1                lpf            480000         45         38         89     21600000
  sine-1               sine           480000         12          9         45      5760000
```

Focus optimization efforts on modules with highest `Total(ns)`.
