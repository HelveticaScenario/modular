// Integration test: verify InjectIndexPtr is generated alongside #[derive(Connect)].
//
// The proc-macro expands to `crate::types::...` and `crate::Patch`,
// so we provide those module shims here.

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
fn inject_index_ptr_impl_exists() {
    use modular_core::InjectIndexPtr;
    use std::cell::Cell;

    let idx = Cell::new(5usize);
    let mut params = TestParams {
        input: Signal::Volts(1.0),
        gain: 0.5,
    };
    // Must compile; inject_index_ptr on a Cable signal wires the pointer.
    params.inject_index_ptr(&idx as *const _);
}

#[test]
fn inject_index_ptr_wires_cable_in_params() {
    use modular_core::{types::WellKnownModule, InjectIndexPtr};
    use std::cell::Cell;

    let idx = Cell::new(3usize);
    let mut params = TestParams {
        input: WellKnownModule::RootClock.to_cable(0, "barTrigger"),
        gain: 1.0,
    };
    params.inject_index_ptr(&idx as *const _);

    if let Signal::Cable { index_ptr, .. } = params.input {
        assert!(!index_ptr.is_null());
        assert_eq!(unsafe { (*index_ptr).get() }, 3);
    } else {
        panic!("expected Cable");
    }
}
