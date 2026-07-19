//! Die Fluent-TestApi des Slice und ihre Fakes — die einzige Black-Box-Fläche, über die die Specs
//! den Slice fahren.

mod fakes;
mod windowing_test_api;

pub use windowing_test_api::WindowingTestApi;
