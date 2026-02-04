//! Performance metrics collection and logging.
//!
//! This module handles:
//! - Collecting timing metrics from the audio thread
//! - Tracking module ID remaps (internal ID → external DSL-assigned ID)
//! - Storing ModuleState registry for params lookup
//! - Writing performance logs to disk

use modular_core::types::{ModuleIdRemap, ModuleState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::ModuleTimingReport;

/// A single performance log entry (JSON-lines format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfLogEntry {
    /// Unix timestamp in seconds
    pub ts: u64,
    /// External module ID (DSL-assigned, after remapping)
    pub module_id: String,
    /// Module type (e.g., "sine", "mix", "seq")
    pub module_type: String,
    /// Module parameters (from ModuleState)
    pub params: serde_json::Value,
    /// Number of update() calls in this period
    pub count: u64,
    /// Total nanoseconds spent in update()
    pub total_ns: u64,
    /// Average nanoseconds per update() call
    pub avg_ns: u64,
    /// Minimum nanoseconds for a single update() call
    pub min_ns: u64,
    /// Maximum nanoseconds for a single update() call
    pub max_ns: u64,
}

/// Manages performance metrics collection and logging
pub struct MetricsManager {
    /// Map from internal module ID (what module stores) → external ID (DSL-assigned)
    /// Updated when patch remaps are applied
    id_remap: HashMap<String, String>,

    /// Registry of ModuleState by external ID, updated on each patch update
    module_registry: HashMap<String, ModuleState>,

    /// Pending metrics that couldn't be resolved yet (internal ID not in remap)
    pending_metrics: Vec<ModuleTimingReport>,

    /// Log file writer (lazy initialized)
    log_writer: Option<BufWriter<File>>,

    /// Path to the log file
    log_path: PathBuf,
}

impl MetricsManager {
    /// Create a new MetricsManager with the given log file path
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            id_remap: HashMap::new(),
            module_registry: HashMap::new(),
            pending_metrics: Vec::new(),
            log_writer: None,
            log_path,
        }
    }

    /// Get the default log file path
    pub fn default_log_path() -> PathBuf {
        // Check for override via environment variable
        if let Ok(override_path) = std::env::var("MODULAR_PERF_LOG") {
            return PathBuf::from(override_path);
        }

        // Use dirs crate for cross-platform data directory
        let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("modular").join("perf.jsonl")
    }

    /// Update the ID remap table and module registry when a new patch is applied.
    /// Called from the main thread when update_patch() receives a new PatchGraph.
    pub fn on_patch_update(&mut self, modules: &[ModuleState], remaps: &[ModuleIdRemap]) {
        // Apply remaps to our internal mapping
        // The remap tells us: module previously known as `from` is now known as `to`
        for remap in remaps {
            // If we had a mapping for `from`, move it to `to`
            if let Some(internal_id) = self.find_internal_id_for_external(&remap.from) {
                let internal_id = internal_id.clone();
                self.id_remap.insert(internal_id, remap.to.clone());
            }
        }

        // Rebuild the module registry from the new module list
        self.module_registry.clear();
        for module_state in modules {
            // The module_state.id is the external (DSL-assigned) ID
            self.module_registry
                .insert(module_state.id.clone(), module_state.clone());

            // For new modules, assume internal ID == external ID initially
            // (will be updated if/when a remap occurs)
            if !self.id_remap.values().any(|v| v == &module_state.id) {
                // Check if this external ID is not yet mapped from any internal ID
                // This means it's a new module, so map it to itself
                self.id_remap
                    .insert(module_state.id.clone(), module_state.id.clone());
            }
        }

        // Clean up id_remap: remove mappings for modules no longer in the registry
        // This prevents logging metrics for modules that have been removed from the patch
        self.id_remap
            .retain(|_, external_id| self.module_registry.contains_key(external_id));

        // Clear pending metrics for removed modules
        self.pending_metrics
            .retain(|report| self.id_remap.contains_key(&report.module_id));

        // Try to process any pending metrics now that we have updated mappings
        self.flush_pending_metrics();
    }

    /// Find the internal ID that maps to a given external ID
    fn find_internal_id_for_external(&self, external_id: &str) -> Option<&String> {
        self.id_remap
            .iter()
            .find(|(_, v)| *v == external_id)
            .map(|(k, _)| k)
    }

    /// Process incoming timing reports from the audio thread.
    /// Resolves internal IDs to external IDs and writes to log.
    pub fn process_metrics(&mut self, reports: Vec<ModuleTimingReport>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for report in reports {
            // Try to resolve internal ID to external ID
            if let Some(external_id) = self.id_remap.get(&report.module_id) {
                // Skip if module is no longer in the current patch
                // (race condition: audio thread may send metrics for a module
                // that was just removed in a patch update)
                if !self.module_registry.contains_key(external_id) {
                    continue;
                }

                // Look up params from registry
                let params = self
                    .module_registry
                    .get(external_id)
                    .map(|m| m.params.clone())
                    .unwrap_or(serde_json::Value::Null);

                let avg_ns = if report.count > 0 {
                    report.total_ns / report.count
                } else {
                    0
                };

                let entry = PerfLogEntry {
                    ts: now,
                    module_id: external_id.clone(),
                    module_type: report.module_type.clone(),
                    params,
                    count: report.count,
                    total_ns: report.total_ns,
                    avg_ns,
                    min_ns: report.min_ns,
                    max_ns: report.max_ns,
                };

                self.write_log_entry(&entry);
            }
            // Don't queue unresolved metrics - if we can't resolve it now,
            // the module likely doesn't exist in the current patch
        }
    }

    /// Try to flush pending metrics that couldn't be resolved earlier
    fn flush_pending_metrics(&mut self) {
        let pending = std::mem::take(&mut self.pending_metrics);
        self.process_metrics(pending);
    }

    /// Write a single log entry to the file
    fn write_log_entry(&mut self, entry: &PerfLogEntry) {
        // Lazy initialize the log writer
        if self.log_writer.is_none() {
            if let Err(e) = self.init_log_writer() {
                eprintln!("Failed to initialize perf log writer: {}", e);
                return;
            }
        }

        if let Some(writer) = &mut self.log_writer {
            match serde_json::to_string(entry) {
                Ok(json) => {
                    if let Err(e) = writeln!(writer, "{}", json) {
                        eprintln!("Failed to write perf log entry: {}", e);
                    }
                    // Flush periodically to ensure data is written
                    let _ = writer.flush();
                }
                Err(e) => {
                    eprintln!("Failed to serialize perf log entry: {}", e);
                }
            }
        }
    }

    /// Initialize the log file writer, creating parent directories if needed
    fn init_log_writer(&mut self) -> std::io::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = self.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        self.log_writer = Some(BufWriter::new(file));
        println!("Performance log: {}", self.log_path.display());

        Ok(())
    }

    /// Get the path to the current log file
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}

impl Default for MetricsManager {
    fn default() -> Self {
        Self::new(Self::default_log_path())
    }
}
