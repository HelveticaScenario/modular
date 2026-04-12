use modular_core::types::{BufferData, BufferSpec};
use std::sync::Arc;

#[derive(Debug)]
pub struct RuntimeBuffer {
  spec: BufferSpec,
  shared: Arc<BufferData>,
}

impl RuntimeBuffer {
  pub fn zeroed(spec: BufferSpec) -> Self {
    Self {
      shared: Arc::new(BufferData::new_zeroed(spec.channels, spec.frame_count)),
      spec,
    }
  }

  pub fn spec(&self) -> &BufferSpec {
    &self.spec
  }

  pub fn name(&self) -> &str {
    &self.spec.name
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
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn zeroed_buffer_has_correct_shape() {
    let spec = BufferSpec::new("test".to_string(), 2, 100).unwrap();
    let buffer = RuntimeBuffer::zeroed(spec);
    assert_eq!(buffer.shared().channel_count(), 2);
    assert_eq!(buffer.shared().frame_count(), 100);
    assert_eq!(buffer.shared().read(0, 0), 0.0);
  }

  #[test]
  fn copy_overlap_from_preserves_old_data_and_leaves_rest_zeroed() {
    let old_spec = BufferSpec::new("old".to_string(), 2, 4).unwrap();
    let new_spec = BufferSpec::new("new".to_string(), 3, 6).unwrap();

    let old = RuntimeBuffer::zeroed(old_spec);
    let new = RuntimeBuffer::zeroed(new_spec);

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
  fn same_shape_compares_channels_and_frames() {
    let a = RuntimeBuffer::zeroed(BufferSpec::new("a".to_string(), 2, 100).unwrap());
    let b = RuntimeBuffer::zeroed(BufferSpec::new("b".to_string(), 2, 100).unwrap());
    let c = RuntimeBuffer::zeroed(BufferSpec::new("c".to_string(), 1, 100).unwrap());
    assert!(a.same_shape(&b));
    assert!(!a.same_shape(&c));
  }
}
