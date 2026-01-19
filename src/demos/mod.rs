/// JerichoOS WASM Demo Module
///
/// Provides canonical test suite for validating WASM runtime functionality

mod wasm_tests;
mod security_tests;

pub use wasm_tests::run_all_demos as run_demos;
pub use security_tests::run_all_security_tests;
