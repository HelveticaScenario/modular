//! Criterion benchmarks for modular_core DSP
//!
//! Run with: cargo bench -p modular_core
//!
//! These benchmarks measure the performance of individual modules and
//! full patch processing to establish baselines and detect regressions.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use modular_core::dsp::get_constructors;
use modular_core::patch::Patch;
use modular_core::types::{ModuleState, PatchGraph, ROOT_ID, ROOT_OUTPUT_PORT};
use std::fs;

const SAMPLE_RATE: f32 = 48000.0;
const FRAMES_PER_ITER: u64 = 480; // 10ms worth

/// Helper to create a ModuleState with default id_is_explicit
fn module(id: &str, module_type: &str, params: serde_json::Value) -> ModuleState {
    ModuleState {
        id: id.to_string(),
        module_type: module_type.to_string(),
        id_is_explicit: None,
        params,
    }
}

/// Build a Patch from a PatchGraph
fn build_patch(graph: &PatchGraph) -> Patch {
    let constructors = get_constructors();
    let mut patch = Patch::new();

    for module_state in &graph.modules {
        if let Some(constructor) = constructors.get(&module_state.module_type) {
            if let Ok(m) = constructor(&module_state.id, SAMPLE_RATE) {
                let _ = m.try_update_params(module_state.params.clone());
                patch.sampleables.insert(module_state.id.clone(), m);
            }
        }
    }

    for m in patch.sampleables.values() {
        m.connect(&patch);
    }
    for m in patch.sampleables.values() {
        m.on_patch_update();
    }
    patch.rebuild_message_listeners();

    patch
}

/// Process N frames through a patch
#[inline(always)]
fn process_frames(patch: &Patch, n: u64) {
    for _ in 0..n {
        for m in patch.sampleables.values() {
            m.update();
        }
        for m in patch.sampleables.values() {
            m.tick();
        }
        // Read output to prevent dead code elimination
        if let Some(root) = patch.sampleables.get(&*ROOT_ID) {
            if let Ok(poly) = root.get_poly_sample(&ROOT_OUTPUT_PORT) {
                black_box(poly.get(0));
            }
        }
    }
}

/// Create a minimal patch with just one module type
fn single_module_patch(module_type: &str, module_id: &str, params: serde_json::Value) -> PatchGraph {
    PatchGraph {
        modules: vec![
            module(
                "ROOT_OUTPUT",
                "signal",
                serde_json::json!({
                    "source": {
                        "Cable": {
                            "module": module_id,
                            "port": "output",
                            "channel": 0
                        }
                    }
                }),
            ),
            module(module_id, module_type, params),
        ],
        module_id_remaps: None,
        scopes: vec![],
    }
}

// ============================================================================
// Individual Module Benchmarks
// ============================================================================

fn bench_sine(c: &mut Criterion) {
    let graph = single_module_patch(
        "sine",
        "sine-1",
        serde_json::json!({ "freq": { "Volts": { "value": 0.0 } } }),
    );
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "sine"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_saw(c: &mut Criterion) {
    let graph = single_module_patch(
        "saw",
        "saw-1",
        serde_json::json!({ "freq": { "Volts": { "value": 0.0 } } }),
    );
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "saw"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_pulse(c: &mut Criterion) {
    let graph = single_module_patch(
        "pulse",
        "pulse-1",
        serde_json::json!({
            "freq": { "Volts": { "value": 0.0 } },
            "width": { "Volts": { "value": 0.5 } }
        }),
    );
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "pulse"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_noise(c: &mut Criterion) {
    let graph = single_module_patch(
        "noise",
        "noise-1",
        serde_json::json!({}),
    );
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "noise"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_lpf(c: &mut Criterion) {
    // LPF needs an input signal
    let graph = PatchGraph {
        modules: vec![
            module(
                "ROOT_OUTPUT",
                "signal",
                serde_json::json!({
                    "source": { "Cable": { "module": "lpf-1", "port": "output", "channel": 0 } }
                }),
            ),
            module(
                "lpf-1",
                "lpf",
                serde_json::json!({
                    "input": { "Cable": { "module": "saw-1", "port": "output", "channel": 0 } },
                    "cutoff": { "Volts": { "value": 0.0 } },
                    "resonance": { "Volts": { "value": 2.0 } }
                }),
            ),
            module(
                "saw-1",
                "saw",
                serde_json::json!({ "freq": { "Volts": { "value": 0.0 } } }),
            ),
        ],
        module_id_remaps: None,
        scopes: vec![],
    };
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "lpf"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_plaits(c: &mut Criterion) {
    let graph = PatchGraph {
        modules: vec![
            module(
                "ROOT_OUTPUT",
                "signal",
                serde_json::json!({
                    "source": { "Cable": { "module": "plaits-1", "port": "out", "channel": 0 } }
                }),
            ),
            module(
                "plaits-1",
                "plaits",
                serde_json::json!({
                    "pitch": { "Volts": { "value": 0.0 } },
                    "engine": "VaVcf",
                    "harmonics": { "Volts": { "value": 0.5 } },
                    "timbre": { "Volts": { "value": 0.5 } },
                    "morph": { "Volts": { "value": 0.5 } }
                }),
            ),
        ],
        module_id_remaps: None,
        scopes: vec![],
    };
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "plaits"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

fn bench_mix(c: &mut Criterion) {
    // Mix with 8 inputs
    let mut modules = vec![
        module(
            "ROOT_OUTPUT",
            "signal",
            serde_json::json!({
                "source": { "Cable": { "module": "mix-1", "port": "output", "channel": 0 } }
            }),
        ),
        module(
            "mix-1",
            "mix",
            serde_json::json!({
                "inputs": [
                    { "Cable": { "module": "sine-1", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-2", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-3", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-4", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-5", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-6", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-7", "port": "output", "channel": 0 } },
                    { "Cable": { "module": "sine-8", "port": "output", "channel": 0 } }
                ],
                "mode": "average",
                "gain": { "Volts": { "value": 1.0 } }
            }),
        ),
    ];

    for i in 1..=8 {
        modules.push(module(
            &format!("sine-{}", i),
            "sine",
            serde_json::json!({ "freq": { "Volts": { "value": (i as f32) * 0.1 } } }),
        ));
    }

    let graph = PatchGraph {
        modules,
        module_id_remaps: None,
        scopes: vec![],
    };
    let patch = build_patch(&graph);

    c.bench_with_input(
        BenchmarkId::new("module", "mix_8inputs"),
        &patch,
        |b, patch| {
            b.iter(|| process_frames(patch, FRAMES_PER_ITER))
        },
    );
}

// ============================================================================
// Patch-level Benchmarks (load from JSON files)
// ============================================================================

fn load_patch_file(name: &str) -> Option<PatchGraph> {
    // Try multiple locations for the patches
    let paths = [
        format!("patches/{}.json", name),
        format!("crates/modular_cli/patches/{}.json", name),
        format!("../modular_cli/patches/{}.json", name),
    ];

    for path in &paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(graph) = serde_json::from_str(&content) {
                return Some(graph);
            }
        }
    }
    None
}

fn bench_patch_files(c: &mut Criterion) {
    let patch_names = ["simple_sine", "poly_stack", "filter_sweep", "plaits_heavy", "full_mix"];

    let mut group = c.benchmark_group("patches");
    group.throughput(Throughput::Elements(FRAMES_PER_ITER));

    for name in patch_names {
        if let Some(graph) = load_patch_file(name) {
            let patch = build_patch(&graph);
            let module_count = graph.modules.len();

            group.bench_with_input(
                BenchmarkId::new(name, module_count),
                &patch,
                |b, patch| {
                    b.iter(|| process_frames(patch, FRAMES_PER_ITER))
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Throughput Benchmark (frames/second capacity)
// ============================================================================

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    // Test with increasing complexity
    let module_counts = [1, 4, 8, 16, 32];

    for count in module_counts {
        // Create a patch with N sine oscillators mixed together
        let mut modules = vec![module(
            "ROOT_OUTPUT",
            "signal",
            serde_json::json!({
                "source": { "Cable": { "module": "mix-1", "port": "output", "channel": 0 } }
            }),
        )];

        let inputs: Vec<serde_json::Value> = (1..=count)
            .map(|i| {
                serde_json::json!({ "Cable": { "module": format!("sine-{}", i), "port": "output", "channel": 0 } })
            })
            .collect();

        modules.push(module(
            "mix-1",
            "mix",
            serde_json::json!({
                "inputs": inputs,
                "mode": "average",
                "gain": { "Volts": { "value": 1.0 } }
            }),
        ));

        for i in 1..=count {
            modules.push(module(
                &format!("sine-{}", i),
                "sine",
                serde_json::json!({ "freq": { "Volts": { "value": (i as f32) * 0.1 } } }),
            ));
        }

        let graph = PatchGraph {
            modules,
            module_id_remaps: None,
            scopes: vec![],
        };

        let patch = build_patch(&graph);

        group.throughput(Throughput::Elements(FRAMES_PER_ITER));
        group.bench_with_input(
            BenchmarkId::new("oscillators", count),
            &patch,
            |b, patch| {
                b.iter(|| process_frames(patch, FRAMES_PER_ITER))
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sine,
    bench_saw,
    bench_pulse,
    bench_noise,
    bench_lpf,
    bench_plaits,
    bench_mix,
    bench_patch_files,
    bench_throughput,
);
criterion_main!(benches);
