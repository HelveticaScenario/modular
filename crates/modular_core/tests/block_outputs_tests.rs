// Integration test: verify {Name}BlockOutputs is generated alongside #[derive(Outputs)].
//
// The proc-macro expands to `crate::types::...`, `crate::block_port::...`, `crate::poly::...`
// so we provide those module shims here.

mod types {
    pub use modular_core::types::*;
}
mod block_port {
    pub use modular_core::block_port::*;
}
mod poly {
    pub use modular_core::poly::*;
}

use modular_core::poly::PolyOutput;

#[derive(modular_derive::Outputs)]
struct SimpleOutputs {
    #[output("value", "A value", default)]
    value: f32,
    #[output("poly", "Poly out")]
    poly: PolyOutput,
}

#[test]
fn block_outputs_struct_exists() {
    let bo = SimpleBlockOutputs::new(4);
    // Fresh buffer returns 0.0
    assert_eq!(bo.get_at("value", 0, 0), 0.0);
    assert_eq!(bo.get_at("poly", 3, 2), 0.0);
}

#[test]
fn copy_from_inner_fills_block_outputs() {
    let inner = SimpleOutputs {
        value: 2.5,
        poly: PolyOutput::mono(1.0),
    };
    let mut bo = SimpleBlockOutputs::new(4);
    bo.copy_from_inner(&inner, 2);
    assert!((bo.get_at("value", 0, 2) - 2.5).abs() < 1e-6);
    assert!((bo.get_at("poly", 0, 2) - 1.0).abs() < 1e-6);
}

#[test]
fn get_at_unknown_port_returns_zero() {
    let bo = SimpleBlockOutputs::new(4);
    assert_eq!(bo.get_at("nonexistent", 0, 0), 0.0);
}
