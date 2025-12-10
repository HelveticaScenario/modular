# Performance Optimizations

This document describes the performance optimizations implemented in the modular synthesizer codebase.

## Overview

The modular synthesizer processes audio in real-time, requiring low-latency and efficient code paths. The audio callback runs on a dedicated thread and must complete processing for each frame within a tight deadline (typically ~1ms at 48kHz sample rate).

## Critical Hot Paths

The following code paths are executed for every audio frame (48,000 times per second at 48kHz):

1. **Audio frame processing** (`modular_server/src/audio.rs::process_frame()`)
2. **Track interpolation** (`modular_core/src/types.rs::InternalTrack::tick()`)
3. **Module DSP updates** (various `update()` methods in `modular_core/src/dsp/`)
4. **Parameter value retrieval** (`InternalParam::get_value()`)

## Implemented Optimizations

### 1. String Allocation Elimination in Audio Thread

**Location**: `modular_server/src/audio.rs:322`

**Before**:
```rust
root.get_sample(&"output".to_string()).unwrap_or(0.0)
```

**After**:
```rust
root.get_sample(&*ROOT_OUTPUT_PORT).unwrap_or(0.0)
```

**Impact**: Eliminates one heap allocation per audio frame (48,000 allocations/second at 48kHz).

### 2. RingBuffer Optimization

**Location**: `modular_server/src/audio.rs:49-66`

**Before**:
```rust
pub fn to_vec(&self) -> Vec<f32> {
    let mut vec = Vec::with_capacity(self.buffer.len());
    for i in 0..self.buffer.len() {
        let idx = (self.index + i) % self.buffer.len();
        vec.push(self.buffer[idx]);
    }
    vec
}
```

**After**:
```rust
pub fn to_vec(&self) -> Vec<f32> {
    if self.buffer.is_empty() {
        return Vec::new();
    }
    
    let len = self.buffer.len();
    let mut vec = Vec::with_capacity(len);
    
    // Optimize by splitting into two slices to avoid modulo on every iteration
    if self.index < len {
        vec.extend_from_slice(&self.buffer[self.index..]);
        vec.extend_from_slice(&self.buffer[..self.index]);
    } else {
        vec.extend_from_slice(&self.buffer);
    }
    
    vec
}
```

**Impact**: 
- Eliminates N modulo operations (where N = buffer size, typically 512)
- Uses efficient slice copying instead of element-by-element push
- Better cache locality with contiguous memory operations

### 3. Binary Search for Keyframe Interpolation

**Location**: `modular_core/src/types.rs:443-462`

**Before**: O(n) linear search through all keyframes
**After**: O(log n) binary search using Rust's `binary_search_by()`

**Impact**: For tracks with many keyframes (e.g., 100), reduces search time from ~100 comparisons to ~7 comparisons per frame.

### 4. Optimized Keyframe Insertion

**Location**: `modular_core/src/types.rs:392-424`

**Before**: Always sorted the entire keyframe vector on every insertion
**After**: 
- For updates: only re-sort if the time value changed
- For new keyframes: use binary search to find insertion position

**Impact**: Reduces keyframe addition from O(n log n) to O(n) in most cases, avoiding unnecessary sorting.

### 5. Reference-Based HashMap Operations

**Location**: `modular_server/src/http_server.rs:385-481`

**Before**: Cloned all module and track IDs into HashSets
```rust
let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();
```

**After**: Use string references to avoid cloning
```rust
let current_ids: HashSet<&str> = patch_lock.sampleables.keys().map(|k| k.as_str()).collect();
let desired_ids: HashSet<&str> = desired_modules.keys().copied().collect();
```

**Impact**: Eliminates hundreds of String allocations during patch updates (typically 10-100 modules per patch).

### 6. Mutex Lock Optimization in Audio Thread

**Location**: `modular_core/src/types.rs:562-577`

**Before**: Used `try_lock_for(Duration::from_millis(10))` with timeout
**After**: Used `try_lock()` with graceful degradation

**Before**:
```rust
pub fn tick(&self) {
    let playhead_value = self
        .playhead_param
        .try_lock_for(Duration::from_millis(10))
        .unwrap()
        .get_value_optional();
    // ... more lock operations with timeouts
}
```

**After**:
```rust
pub fn tick(&self) {
    let playhead_value = match self.playhead_param.try_lock() {
        Some(guard) => guard.get_value_optional(),
        None => return, // Keep previous sample if locked
    };
    // ... graceful degradation on contention
}
```

**Impact**: 
- Eliminates timeout mechanism overhead
- Provides instant lock attempt with zero blocking
- Gracefully degrades by keeping previous value on contention
- Critical for real-time audio thread performance

## Best Practices

### For Audio Thread Code

1. **Never block**: Always use `try_lock()` instead of `lock()` or `try_lock_for()`
2. **Avoid allocations**: Pre-allocate buffers and reuse them
3. **Minimize string operations**: Use `&str` references, avoid `to_string()` and `clone()`
4. **Cache expensive calculations**: Store results of `powf()`, `sin()`, etc. when possible
5. **Use efficient data structures**: Binary search for sorted data, ring buffers for streaming

### For Non-Audio Code (Server, Validation, etc.)

1. **Use references**: Prefer `&str` over `String` when building temporary collections
2. **Lazy evaluation**: Only compute values when needed
3. **Batch operations**: Group related updates to minimize lock contention
4. **Profile before optimizing**: Measure to identify actual bottlenecks

## Measurement and Profiling

To profile the audio code:

```bash
# Build with release optimizations
cargo build --release --package modular_server

# Run with profiling (requires perf on Linux)
perf record -g ./target/release/modular_server
perf report
```

For detailed profiling, consider using:
- `cargo-flamegraph` for flame graphs
- `valgrind --tool=callgrind` for detailed call graphs
- Built-in audio buffer underrun detection (monitor console for warnings)

## Future Optimization Opportunities

1. **SIMD vectorization**: Use SIMD instructions for parallel DSP operations
2. **Filter coefficient caching**: Cache filter coefficients when parameters haven't changed significantly
3. **Lock-free data structures**: Replace some Mutex usage with lock-free alternatives for subscriptions
4. **Pre-computed lookup tables**: Expand use of LUTs for expensive math operations
5. **Memory pool**: Implement custom allocator for audio-thread temporary buffers

## Performance Monitoring

Key metrics to watch:

- **Audio buffer underruns**: Indicates audio thread is too slow
- **Lock contention**: Monitor `try_lock()` failure rates in tracks
- **Allocation rate**: Minimize allocations in audio thread
- **CPU usage**: Should remain below ~30% to allow headroom for other processes

## Related Documentation

- [Architecture Overview](../README.md)
- [DSP Module Development](DSL_GUIDE.md)
- [Audio Streaming Implementation](../AUDIO_STREAMING_IMPLEMENTATION.md)
