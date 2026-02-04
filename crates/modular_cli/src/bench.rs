//! modular-bench: Benchmark harness for DSP profiling
//!
//! This binary allows profiling the Rust DSP code without N-API or cpal,
//! enabling use of native profiling tools like samply, Instruments, or perf.
//!
//! Usage:
//!   modular-bench run patches/simple_sine.json --frames 1000000
//!   modular-bench list
//!   samply record ./target/profiling/modular-bench run patches/full_mix.json

use clap::{Parser, Subcommand};
use modular_core::dsp::get_constructors;
use modular_core::patch::Patch;
use modular_core::types::PatchGraph;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(feature = "profile")]
use tracing_subscriber::layer::SubscriberExt;

const DEFAULT_SAMPLE_RATE: f32 = 48000.0;
const DEFAULT_FRAMES: u64 = 48000 * 10; // 10 seconds at 48kHz

/// Benchmark harness for modular synthesizer DSP profiling
#[derive(Parser)]
#[command(name = "modular-bench")]
#[command(about = "Profile and benchmark the modular synthesizer DSP engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a benchmark with a patch file
    Run {
        /// Path to the patch JSON file
        patch: PathBuf,

        /// Number of audio frames to process
        #[arg(short, long, default_value_t = DEFAULT_FRAMES)]
        frames: u64,

        /// Sample rate in Hz
        #[arg(short, long, default_value_t = DEFAULT_SAMPLE_RATE)]
        sample_rate: f32,

        /// Number of output channels
        #[arg(short, long, default_value_t = 2)]
        channels: usize,

        /// Warmup frames before measurement
        #[arg(short, long, default_value_t = 48000)]
        warmup: u64,

        /// Print per-module timing stats
        #[arg(long)]
        stats: bool,
    },

    /// List available benchmark patches
    List,

    /// Run a quick smoke test with all patches
    Smoke {
        /// Frames per patch for smoke test
        #[arg(short, long, default_value_t = 4800)]
        frames: u64,
    },
}

fn main() {
    // Initialize Tracy if profile feature is enabled
    #[cfg(feature = "profile")]
    {
        use tracing_subscriber::prelude::*;
        let tracy_layer = tracing_tracy::TracyLayer::default();
        tracing_subscriber::registry().with(tracy_layer).init();
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            patch,
            frames,
            sample_rate,
            channels,
            warmup,
            stats,
        } => {
            run_benchmark(&patch, frames, sample_rate, channels, warmup, stats);
        }
        Commands::List => {
            list_patches();
        }
        Commands::Smoke { frames } => {
            smoke_test(frames);
        }
    }
}

/// Build a Patch from a PatchGraph JSON
fn build_patch(graph: &PatchGraph, sample_rate: f32) -> Result<Patch, String> {
    let constructors = get_constructors();
    let mut patch = Patch::new();

    // Create all modules
    for module_state in &graph.modules {
        let constructor = constructors
            .get(&module_state.module_type)
            .ok_or_else(|| format!("Unknown module type: {}", module_state.module_type))?;

        let module = constructor(&module_state.id, sample_rate)
            .map_err(|e| format!("Failed to create module {}: {}", module_state.id, e))?;

        // Apply params
        module
            .try_update_params(module_state.params.clone())
            .map_err(|e| format!("Failed to update params for {}: {}", module_state.id, e))?;

        patch.sampleables.insert(module_state.id.clone(), module);
    }

    // Connect all modules
    for module in patch.sampleables.values() {
        module.connect(&patch);
    }

    // Call on_patch_update for all modules
    for module in patch.sampleables.values() {
        module.on_patch_update();
    }

    patch.rebuild_message_listeners();

    Ok(patch)
}

/// Process a single frame
#[inline(always)]
fn process_frame(patch: &Patch) {
    #[cfg(feature = "profile")]
    let _span = tracing::info_span!("process_frame").entered();

    // Update all modules
    {
        #[cfg(feature = "profile")]
        let _span = tracing::info_span!("update_modules").entered();
        for module in patch.sampleables.values() {
            module.update();
        }
    }

    // Tick all modules
    {
        #[cfg(feature = "profile")]
        let _span = tracing::info_span!("tick_modules").entered();
        for module in patch.sampleables.values() {
            module.tick();
        }
    }
}

/// Get output from root module
fn get_output(patch: &Patch, channels: usize) -> [f32; 16] {
    use modular_core::types::{ROOT_ID, ROOT_OUTPUT_PORT};
    let mut output = [0.0f32; 16];

    if let Some(root) = patch.sampleables.get(&*ROOT_ID) {
        if let Ok(poly) = root.get_poly_sample(&ROOT_OUTPUT_PORT) {
            for ch in 0..channels.min(16) {
                output[ch] = poly.get(ch);
            }
        }
    }

    output
}

fn run_benchmark(
    patch_path: &PathBuf,
    frames: u64,
    sample_rate: f32,
    channels: usize,
    warmup: u64,
    print_stats: bool,
) {
    // Load patch
    let patch_json = fs::read_to_string(patch_path)
        .unwrap_or_else(|e| panic!("Failed to read patch file {:?}: {}", patch_path, e));

    let graph: PatchGraph = serde_json::from_str(&patch_json)
        .unwrap_or_else(|e| panic!("Failed to parse patch JSON: {}", e));

    println!("Loaded patch: {} modules", graph.modules.len());
    for module in &graph.modules {
        println!("  - {} ({})", module.id, module.module_type);
    }

    // Build patch
    let patch = build_patch(&graph, sample_rate).unwrap_or_else(|e| panic!("{}", e));

    println!(
        "\nRunning benchmark: {} frames ({:.2}s at {}Hz)",
        frames,
        frames as f64 / sample_rate as f64,
        sample_rate
    );
    println!("  Warmup: {} frames", warmup);
    println!("  Channels: {}", channels);

    // Warmup phase
    print!("Warming up...");
    for _ in 0..warmup {
        process_frame(&patch);
        black_box(get_output(&patch, channels));
    }
    println!(" done");

    // Reset timing metrics after warmup
    for module in patch.sampleables.values() {
        module.reset_timing_metrics();
    }

    // Benchmark phase
    print!("Benchmarking...");
    let start = Instant::now();

    for _ in 0..frames {
        process_frame(&patch);
        black_box(get_output(&patch, channels));
    }

    let elapsed = start.elapsed();
    println!(" done\n");

    // Results
    let total_ns = elapsed.as_nanos() as f64;
    let ns_per_frame = total_ns / frames as f64;
    let frames_per_sec = 1_000_000_000.0 / ns_per_frame;
    let realtime_budget_ns = 1_000_000_000.0 / sample_rate as f64;
    let budget_usage = (ns_per_frame / realtime_budget_ns) * 100.0;

    println!("Results:");
    println!("  Total time:     {:?}", elapsed);
    println!("  Frames:         {}", frames);
    println!("  ns/frame:       {:.2}", ns_per_frame);
    println!("  frames/sec:     {:.0}", frames_per_sec);
    println!(
        "  Real-time budget: {:.2} ns/frame @ {}Hz",
        realtime_budget_ns, sample_rate
    );
    println!("  Budget usage:   {:.2}%", budget_usage);

    if budget_usage > 100.0 {
        println!("\n  ⚠️  WARNING: Exceeds real-time budget!");
    } else {
        println!(
            "\n  ✓ Within real-time budget ({:.1}x headroom)",
            100.0 / budget_usage
        );
    }

    // Per-module stats
    if print_stats {
        println!("\nPer-module timing:");
        let mut module_stats: Vec<_> = patch
            .sampleables
            .values()
            .filter_map(|m| {
                m.get_timing_metrics().map(|(total, count, min, max)| {
                    let avg = if count > 0 { total / count } else { 0 };
                    (
                        m.get_id().to_string(),
                        m.get_module_type().to_string(),
                        count,
                        avg,
                        min,
                        max,
                        total,
                    )
                })
            })
            .collect();

        // Sort by total time descending
        module_stats.sort_by(|a, b| b.6.cmp(&a.6));

        println!(
            "  {:20} {:12} {:>10} {:>10} {:>10} {:>10} {:>12}",
            "Module ID", "Type", "Count", "Avg(ns)", "Min(ns)", "Max(ns)", "Total(ns)"
        );
        println!(
            "  {:-<20} {:-<12} {:-<10} {:-<10} {:-<10} {:-<10} {:-<12}",
            "", "", "", "", "", "", ""
        );

        for (id, typ, count, avg, min, max, total) in module_stats {
            println!(
                "  {:20} {:12} {:>10} {:>10} {:>10} {:>10} {:>12}",
                id, typ, count, avg, min, max, total
            );
        }
    }
}

fn get_patches_dir() -> PathBuf {
    // Look for patches relative to the binary or in standard locations
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    let candidates = [
        // Relative to cwd
        PathBuf::from("patches"),
        PathBuf::from("crates/modular_cli/patches"),
        // Relative to exe
        exe_dir
            .clone()
            .map(|p| p.join("patches"))
            .unwrap_or_default(),
        exe_dir
            .map(|p| p.join("../../../crates/modular_cli/patches"))
            .unwrap_or_default(),
    ];

    for path in &candidates {
        if path.exists() && path.is_dir() {
            return path.clone();
        }
    }

    // Default to cwd/patches
    PathBuf::from("crates/modular_cli/patches")
}

fn list_patches() {
    let patches_dir = get_patches_dir();
    println!("Patches directory: {:?}", patches_dir);

    if !patches_dir.exists() {
        println!("  (directory does not exist - create patches here)");
        return;
    }

    let entries: Vec<_> = fs::read_dir(&patches_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    if entries.is_empty() {
        println!("  (no .json patches found)");
        return;
    }

    println!("\nAvailable patches:");
    for entry in entries {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        // Try to load and show module count
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(graph) = serde_json::from_str::<PatchGraph>(&content) {
                println!("  {} ({} modules)", name, graph.modules.len());
            } else {
                println!("  {} (invalid JSON)", name);
            }
        } else {
            println!("  {} (unreadable)", name);
        }
    }
}

fn smoke_test(frames: u64) {
    let patches_dir = get_patches_dir();
    println!("Running smoke test with {} frames per patch\n", frames);

    if !patches_dir.exists() {
        println!("No patches directory found at {:?}", patches_dir);
        return;
    }

    let entries: Vec<_> = fs::read_dir(&patches_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();

    for entry in entries {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        print!("Testing {}... ", name);

        match fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<PatchGraph>(&s).map_err(|e| e.to_string()))
            .and_then(|g| build_patch(&g, DEFAULT_SAMPLE_RATE))
        {
            Ok(patch) => {
                let start = Instant::now();
                for _ in 0..frames {
                    process_frame(&patch);
                    black_box(get_output(&patch, 2));
                }
                let elapsed = start.elapsed();
                let ns_per_frame = elapsed.as_nanos() as f64 / frames as f64;
                println!("OK ({:.2} ns/frame)", ns_per_frame);
            }
            Err(e) => {
                println!("FAILED: {}", e);
            }
        }
    }
}
