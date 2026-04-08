use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use modular_core::types::{BufferData, BufferSpec};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct RuntimeBuffer {
  spec: BufferSpec,
  sample_rate: u32,
  disk_path: PathBuf,
  shared: Arc<BufferData>,
  flush_on_drop: AtomicBool,
}

impl RuntimeBuffer {
  pub fn load_or_zero(spec: BufferSpec, sample_rate: u32) -> Result<Self, String> {
    Self::from_spec(spec, sample_rate, true)
  }

  pub fn zeroed(spec: BufferSpec, sample_rate: u32) -> Result<Self, String> {
    Self::from_spec(spec, sample_rate, false)
  }

  fn from_spec(spec: BufferSpec, sample_rate: u32, load_from_disk: bool) -> Result<Self, String> {
    let disk_path = PathBuf::from(&spec.path);
    let shared = if load_from_disk {
      Arc::new(BufferData::from_samples(load_samples(&disk_path, &spec)?))
    } else {
      Arc::new(BufferData::new_zeroed(spec.channels, spec.frame_count))
    };

    Ok(Self {
      spec,
      sample_rate,
      disk_path,
      shared,
      flush_on_drop: AtomicBool::new(false),
    })
  }

  pub fn spec(&self) -> &BufferSpec {
    &self.spec
  }

  pub fn path(&self) -> &str {
    &self.spec.path
  }

  pub fn shared(&self) -> Arc<BufferData> {
    self.shared.clone()
  }

  pub fn same_shape(&self, other: &Self) -> bool {
    self.spec.same_shape(&other.spec)
  }

  pub fn copy_overlap_from(&self, other: &Self) {
    self.shared.copy_overlap_from(&other.shared);
  }

  pub fn enable_flush_on_drop(&self) {
    self.flush_on_drop.store(true, Ordering::Release);
  }

  pub fn suppress_flush_on_drop(&self) {
    self.flush_on_drop.store(false, Ordering::Release);
  }
}

impl Drop for RuntimeBuffer {
  fn drop(&mut self) {
    if !self.flush_on_drop.load(Ordering::Acquire) {
      return;
    }

    if let Err(err) = flush_buffer(&self.disk_path, self.sample_rate, &self.shared) {
      eprintln!("Failed to flush buffer '{}' to disk: {}", self.disk_path.display(), err);
    }
  }
}

fn load_samples(path: &Path, spec: &BufferSpec) -> Result<Vec<Vec<f32>>, String> {
  let mut output = vec![vec![0.0; spec.frame_count]; spec.channels];
  if !path.exists() {
    return Ok(output);
  }

  let mut reader = WavReader::open(path)
    .map_err(|err| format!("Failed to open WAV '{}': {}", path.display(), err))?;
  let wav_spec = reader.spec();
  let source_channels = wav_spec.channels as usize;
  if source_channels == 0 {
    return Ok(output);
  }

  match wav_spec.sample_format {
    SampleFormat::Float => {
      for (sample_index, sample) in reader.samples::<f32>().enumerate() {
        let sample = sample
          .map_err(|err| format!("Failed to read float sample from '{}': {}", path.display(), err))?;
        let channel = sample_index % source_channels;
        let frame = sample_index / source_channels;
        if channel < spec.channels && frame < spec.frame_count {
          output[channel][frame] = sample;
        }
      }
    }
    SampleFormat::Int => {
      let scale = (((1u64 << (wav_spec.bits_per_sample.saturating_sub(1) as u32))
        .saturating_sub(1)) as f32)
        .max(1.0);
      if wav_spec.bits_per_sample <= 16 {
        for (sample_index, sample) in reader.samples::<i16>().enumerate() {
          let sample = sample.map_err(|err| {
            format!("Failed to read integer sample from '{}': {}", path.display(), err)
          })?;
          let channel = sample_index % source_channels;
          let frame = sample_index / source_channels;
          if channel < spec.channels && frame < spec.frame_count {
            output[channel][frame] = sample as f32 / scale;
          }
        }
      } else {
        for (sample_index, sample) in reader.samples::<i32>().enumerate() {
          let sample = sample.map_err(|err| {
            format!("Failed to read integer sample from '{}': {}", path.display(), err)
          })?;
          let channel = sample_index % source_channels;
          let frame = sample_index / source_channels;
          if channel < spec.channels && frame < spec.frame_count {
            output[channel][frame] = sample as f32 / scale;
          }
        }
      }
    }
  }

  Ok(output)
}

fn flush_buffer(path: &Path, sample_rate: u32, shared: &Arc<BufferData>) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|err| format!("Failed to create buffer directory '{}': {}", parent.display(), err))?;
  }

  let mut writer = WavWriter::create(
    path,
    WavSpec {
      channels: shared.channel_count() as u16,
      sample_rate,
      bits_per_sample: 32,
      sample_format: SampleFormat::Float,
    },
  )
  .map_err(|err| format!("Failed to create WAV '{}': {}", path.display(), err))?;

  let snapshot = shared.snapshot();
  for frame in 0..shared.frame_count() {
    for channel in 0..shared.channel_count() {
      writer
        .write_sample(snapshot[channel][frame])
        .map_err(|err| format!("Failed to write sample to '{}': {}", path.display(), err))?;
    }
  }

  writer
    .finalize()
    .map_err(|err| format!("Failed to finalize WAV '{}': {}", path.display(), err))?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    std::env::temp_dir().join(format!("modular_buffer_{}_{}_{}", std::process::id(), nanos, name))
  }

  fn write_test_wav(path: &Path, channels: u16, frames: &[Vec<f32>]) {
    let mut writer = WavWriter::create(
      path,
      WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
      },
    )
    .unwrap();

    let frame_count = frames.first().map_or(0, Vec::len);
    for frame in 0..frame_count {
      for channel in 0..channels as usize {
        writer.write_sample(frames[channel][frame]).unwrap();
      }
    }
    writer.finalize().unwrap();
  }

  #[test]
  fn load_or_zero_adapts_existing_wav_to_requested_shape() {
    let path = unique_temp_path("adapt.wav");
    write_test_wav(
      &path,
      2,
      &[vec![1.0, 2.0, 3.0, 4.0], vec![10.0, 20.0, 30.0, 40.0]],
    );

    let spec = BufferSpec::new(path.to_string_lossy().to_string(), 1, 6).unwrap();
    let buffer = RuntimeBuffer::load_or_zero(spec, 48_000).unwrap();
    let snapshot = buffer.shared().snapshot();

    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0], vec![1.0, 2.0, 3.0, 4.0, 0.0, 0.0]);

    let _ = fs::remove_file(path);
  }

  #[test]
  fn copy_overlap_from_preserves_old_data_and_leaves_rest_zeroed() {
    let old_spec = BufferSpec::new(unique_temp_path("old.wav").to_string_lossy().to_string(), 2, 4)
      .unwrap();
    let new_spec = BufferSpec::new(unique_temp_path("new.wav").to_string_lossy().to_string(), 3, 6)
      .unwrap();

    let old = RuntimeBuffer::zeroed(old_spec, 48_000).unwrap();
    let new = RuntimeBuffer::zeroed(new_spec, 48_000).unwrap();

    old.shared().write(0, 0, 1.25);
    old.shared().write(1, 3, 2.5);

    new.copy_overlap_from(&old);
    let snapshot = new.shared().snapshot();
    assert_eq!(snapshot[0][0], 1.25);
    assert_eq!(snapshot[1][3], 2.5);
    assert_eq!(snapshot[2][0], 0.0);
    assert_eq!(snapshot[0][5], 0.0);
  }

  #[test]
  fn flush_on_drop_writes_wav_to_disk() {
    let path = unique_temp_path("flush.wav");
    let spec = BufferSpec::new(path.to_string_lossy().to_string(), 1, 4).unwrap();

    {
      let buffer = RuntimeBuffer::zeroed(spec.clone(), 48_000).unwrap();
      buffer.shared().write(0, 1, 0.75);
      buffer.enable_flush_on_drop();
    }

    assert!(path.exists());

    let loaded = RuntimeBuffer::load_or_zero(spec, 48_000).unwrap();
    let snapshot = loaded.shared().snapshot();
    assert_eq!(snapshot[0][1], 0.75);

    let _ = fs::remove_file(path);
  }

  #[test]
  fn suppress_flush_on_drop_skips_disk_write() {
    let path = unique_temp_path("skip.wav");
    let spec = BufferSpec::new(path.to_string_lossy().to_string(), 1, 4).unwrap();

    {
      let buffer = RuntimeBuffer::zeroed(spec, 48_000).unwrap();
      buffer.enable_flush_on_drop();
      buffer.suppress_flush_on_drop();
    }

    assert!(!path.exists());
  }
}
