//! Die Fluent-TestApi des Slice und ihre Fakes — die einzige Black-Box-Fläche, über die die Specs
//! den Slice fahren.

mod fakes;
mod obfuscation_test_api;

pub use obfuscation_test_api::ObfuscationTestApi;
