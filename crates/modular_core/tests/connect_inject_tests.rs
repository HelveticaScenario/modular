// Integration test: verify the unified `Connect` trait wires both module_ptr
// and index_ptr in one call (previously this was split between Connect +
// InjectIndexPtr).
//
// The proc-macro expands to `crate::types::...` and `crate::Patch`, so we
// provide those module shims here.

mod types {
    pub use modular_core::types::*;
}
// Connect derive generates `crate::Patch` (not `crate::patch::Patch`).
use modular_core::patch::Patch;

use modular_core::types::Signal;

#[derive(modular_derive::Connect)]
struct TestParams {
    input: Signal,
    gain: f32,
}

#[test]
fn connect_runs_on_volts_signal_without_panic() {
    use modular_core::types::Connect;
    use std::cell::Cell;

    let idx = Cell::new(5usize);
    let patch = Patch::new();
    let mut params = TestParams {
        input: Signal::Volts(1.0),
        gain: 0.5,
    };
    // Must compile + run; Connect on a Volts signal is a no-op cable-wise.
    params.connect(&patch, &idx as *const _);
}

#[test]
fn connect_wires_index_ptr_into_cable_in_params() {
    use modular_core::types::{Connect, WellKnownModule};
    use std::cell::Cell;

    let idx = Cell::new(3usize);
    let patch = Patch::new();
    let mut params = TestParams {
        input: WellKnownModule::RootClock.to_cable(0, "barTrigger"),
        gain: 1.0,
    };
    params.connect(&patch, &idx as *const _);

    if let Signal::Cable { index_ptr, .. } = params.input {
        assert!(!index_ptr.is_null());
        assert_eq!(unsafe { (*index_ptr).get() }, 3);
    } else {
        panic!("expected Cable");
    }
}
