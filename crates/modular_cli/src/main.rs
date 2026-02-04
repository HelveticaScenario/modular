//! modular-perf: CLI tool for exploring and querying performance logs
//!
//! The modular synthesizer writes performance metrics to a JSON-lines log file.
//! This tool provides commands to analyze those logs:
//!
//! - `tail`: Follow the log file in real-time
//! - `query`: Filter entries by module type, time range, etc.
//! - `top`: Show modules sorted by average execution time
//! - `summary`: Aggregate statistics per module type

use chrono::{TimeZone, Utc};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Performance log entry (matches PerfLogEntry in metrics.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerfLogEntry {
    ts: u64,
    module_id: String,
    module_type: String,
    params: serde_json::Value,
    count: u64,
    total_ns: u64,
    avg_ns: u64,
    min_ns: u64,
    max_ns: u64,
}

/// CLI tool for exploring modular synthesizer performance logs
#[derive(Parser)]
#[command(name = "modular-perf")]
#[command(about = "Analyze performance metrics from the modular synthesizer")]
#[command(version)]
struct Cli {
    /// Path to the performance log file (default: auto-detect)
    #[arg(short, long, global = true)]
    log_file: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "table")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Follow the log file in real-time (like tail -f)
    Tail {
        /// Number of recent entries to show before following
        #[arg(short = 'n', long, default_value = "10")]
        lines: usize,
    },

    /// Query and filter log entries
    Query {
        /// Filter by module type (e.g., "sine", "mix")
        #[arg(long)]
        module_type: Option<String>,

        /// Filter by module ID prefix
        #[arg(long)]
        module_id: Option<String>,

        /// Minimum average time in nanoseconds
        #[arg(long)]
        min_avg: Option<u64>,

        /// Maximum number of entries to show
        #[arg(short = 'n', long, default_value = "100")]
        limit: usize,
    },

    /// Show modules sorted by average execution time (descending)
    Top {
        /// Number of entries to show
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Group by module type instead of instance
        #[arg(long)]
        by_type: bool,
    },

    /// Show aggregate statistics per module type
    Summary,
}

fn default_log_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("MODULAR_PERF_LOG") {
        return PathBuf::from(override_path);
    }

    let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("modular").join("perf.jsonl")
}

fn format_ns(ns: u64) -> String {
    if ns >= 1_000_000 {
        format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else if ns >= 1_000 {
        format!("{:.2}µs", ns as f64 / 1_000.0)
    } else {
        format!("{}ns", ns)
    }
}

fn format_timestamp(ts: u64) -> String {
    Utc.timestamp_opt(ts as i64, 0)
        .single()
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn print_entry_table(entry: &PerfLogEntry) {
    let time = format_timestamp(entry.ts);
    let module_type = entry.module_type.cyan();
    let module_id = entry.module_id.white();
    let avg = format_ns(entry.avg_ns).yellow();
    let min = format_ns(entry.min_ns).green();
    let max = format_ns(entry.max_ns).red();
    let count = entry.count.to_string().dimmed();

    println!(
        "{} {:>12} {:>20} avg={:>10} min={:>10} max={:>10} n={}",
        time, module_type, module_id, avg, min, max, count
    );
}

fn print_entry_json(entry: &PerfLogEntry) {
    if let Ok(json) = serde_json::to_string(entry) {
        println!("{}", json);
    }
}

fn read_entries(path: &PathBuf) -> Vec<PerfLogEntry> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
            return Vec::new();
        }
    };

    let reader = BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

fn cmd_tail(path: &PathBuf, lines: usize, format: OutputFormat) {
    println!("Following: {}", path.display());
    println!("{}", "-".repeat(80));

    // Read existing entries and show last N
    let entries = read_entries(path);
    let start = entries.len().saturating_sub(lines);
    for entry in entries.iter().skip(start) {
        match format {
            OutputFormat::Table => print_entry_table(entry),
            OutputFormat::Json => print_entry_json(entry),
        }
    }

    // Now follow the file
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
            return;
        }
    };

    let mut reader = BufReader::new(file);
    // Seek to end
    if let Err(e) = reader.seek(SeekFrom::End(0)) {
        eprintln!("Failed to seek to end: {}", e);
        return;
    }

    let mut line = String::new();
    loop {
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new data, wait a bit
                thread::sleep(Duration::from_millis(100));
            }
            Ok(_) => {
                if let Ok(entry) = serde_json::from_str::<PerfLogEntry>(&line) {
                    match format {
                        OutputFormat::Table => print_entry_table(&entry),
                        OutputFormat::Json => print_entry_json(&entry),
                    }
                }
                line.clear();
            }
            Err(e) => {
                eprintln!("Error reading log: {}", e);
                break;
            }
        }
    }
}

fn cmd_query(
    path: &PathBuf,
    module_type: Option<String>,
    module_id: Option<String>,
    min_avg: Option<u64>,
    limit: usize,
    format: OutputFormat,
) {
    let entries = read_entries(path);

    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            if let Some(ref mt) = module_type {
                if !e.module_type.contains(mt) {
                    return false;
                }
            }
            if let Some(ref mid) = module_id {
                if !e.module_id.starts_with(mid) {
                    return false;
                }
            }
            if let Some(min) = min_avg {
                if e.avg_ns < min {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .collect();

    if format == OutputFormat::Table {
        println!(
            "{:>8} {:>12} {:>20} {:>10} {:>10} {:>10} {:>8}",
            "TIME", "TYPE", "ID", "AVG", "MIN", "MAX", "COUNT"
        );
        println!("{}", "-".repeat(90));
    }

    for entry in &filtered {
        match format {
            OutputFormat::Table => print_entry_table(entry),
            OutputFormat::Json => print_entry_json(entry),
        }
    }

    if format == OutputFormat::Table {
        println!("{}", "-".repeat(90));
        println!("Showing {} entries", filtered.len());
    }
}

fn cmd_top(path: &PathBuf, limit: usize, by_type: bool, format: OutputFormat) {
    let entries = read_entries(path);

    if by_type {
        // Aggregate by module type
        let mut type_stats: HashMap<String, (u64, u64, u64, u64)> = HashMap::new();

        for entry in &entries {
            let stats = type_stats
                .entry(entry.module_type.clone())
                .or_insert((0, 0, u64::MAX, 0));
            stats.0 += entry.total_ns;
            stats.1 += entry.count;
            stats.2 = stats.2.min(entry.min_ns);
            stats.3 = stats.3.max(entry.max_ns);
        }

        let mut sorted: Vec<_> = type_stats
            .into_iter()
            .map(|(module_type, (total_ns, count, min_ns, max_ns))| {
                let avg_ns = if count > 0 { total_ns / count } else { 0 };
                (module_type, avg_ns, min_ns, max_ns, count)
            })
            .collect();

        sorted.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by avg descending

        if format == OutputFormat::Table {
            println!(
                "{:>20} {:>12} {:>12} {:>12} {:>12}",
                "MODULE TYPE", "AVG", "MIN", "MAX", "TOTAL CALLS"
            );
            println!("{}", "-".repeat(70));
        }

        for (module_type, avg_ns, min_ns, max_ns, count) in sorted.iter().take(limit) {
            match format {
                OutputFormat::Table => {
                    println!(
                        "{:>20} {:>12} {:>12} {:>12} {:>12}",
                        module_type.cyan(),
                        format_ns(*avg_ns).yellow(),
                        format_ns(*min_ns).green(),
                        format_ns(*max_ns).red(),
                        count.to_string().dimmed()
                    );
                }
                OutputFormat::Json => {
                    let obj = serde_json::json!({
                        "module_type": module_type,
                        "avg_ns": avg_ns,
                        "min_ns": min_ns,
                        "max_ns": max_ns,
                        "count": count
                    });
                    println!("{}", obj);
                }
            }
        }
    } else {
        // Show individual entries sorted by avg
        let mut sorted = entries;
        sorted.sort_by(|a, b| b.avg_ns.cmp(&a.avg_ns));

        if format == OutputFormat::Table {
            println!(
                "{:>8} {:>12} {:>20} {:>10} {:>10} {:>10} {:>8}",
                "TIME", "TYPE", "ID", "AVG", "MIN", "MAX", "COUNT"
            );
            println!("{}", "-".repeat(90));
        }

        for entry in sorted.iter().take(limit) {
            match format {
                OutputFormat::Table => print_entry_table(entry),
                OutputFormat::Json => print_entry_json(entry),
            }
        }
    }
}

fn cmd_summary(path: &PathBuf, format: OutputFormat) {
    let entries = read_entries(path);

    // Aggregate by module type
    let mut type_stats: HashMap<String, (u64, u64, u64, u64, usize)> = HashMap::new();

    for entry in &entries {
        let stats = type_stats
            .entry(entry.module_type.clone())
            .or_insert((0, 0, u64::MAX, 0, 0));
        stats.0 += entry.total_ns;
        stats.1 += entry.count;
        stats.2 = stats.2.min(entry.min_ns);
        stats.3 = stats.3.max(entry.max_ns);
        stats.4 += 1; // Number of log entries (time periods)
    }

    let mut sorted: Vec<_> = type_stats.into_iter().collect();
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0)); // Sort by total time descending

    if format == OutputFormat::Table {
        println!(
            "{:>20} {:>12} {:>12} {:>12} {:>12} {:>8}",
            "MODULE TYPE", "AVG", "MIN", "MAX", "TOTAL CALLS", "PERIODS"
        );
        println!("{}", "-".repeat(80));
    }

    let mut grand_total_ns: u64 = 0;
    let mut grand_total_calls: u64 = 0;

    for (module_type, (total_ns, count, min_ns, max_ns, periods)) in &sorted {
        let avg_ns = if *count > 0 { total_ns / count } else { 0 };
        grand_total_ns += total_ns;
        grand_total_calls += count;

        match format {
            OutputFormat::Table => {
                println!(
                    "{:>20} {:>12} {:>12} {:>12} {:>12} {:>8}",
                    module_type.cyan(),
                    format_ns(avg_ns).yellow(),
                    format_ns(*min_ns).green(),
                    format_ns(*max_ns).red(),
                    count.to_string().dimmed(),
                    periods.to_string().dimmed()
                );
            }
            OutputFormat::Json => {
                let obj = serde_json::json!({
                    "module_type": module_type,
                    "total_ns": total_ns,
                    "avg_ns": avg_ns,
                    "min_ns": min_ns,
                    "max_ns": max_ns,
                    "count": count,
                    "periods": periods
                });
                println!("{}", obj);
            }
        }
    }

    if format == OutputFormat::Table {
        println!("{}", "-".repeat(80));
        println!(
            "Total: {} module types, {} calls, {} total time",
            sorted.len(),
            grand_total_calls,
            format_ns(grand_total_ns)
        );
    }
}

fn main() {
    let cli = Cli::parse();

    let log_path = cli.log_file.unwrap_or_else(default_log_path);

    if !log_path.exists() {
        eprintln!("Log file not found: {}", log_path.display());
        eprintln!("The log file is created when you run the modular synthesizer.");
        eprintln!(
            "Default location: {}",
            default_log_path().display()
        );
        eprintln!("Override with: --log-file <path> or MODULAR_PERF_LOG env var");
        std::process::exit(1);
    }

    match cli.command {
        Commands::Tail { lines } => cmd_tail(&log_path, lines, cli.format),
        Commands::Query {
            module_type,
            module_id,
            min_avg,
            limit,
        } => cmd_query(&log_path, module_type, module_id, min_avg, limit, cli.format),
        Commands::Top { limit, by_type } => cmd_top(&log_path, limit, by_type, cli.format),
        Commands::Summary => cmd_summary(&log_path, cli.format),
    }
}
