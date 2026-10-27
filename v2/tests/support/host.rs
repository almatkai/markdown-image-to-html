// Upstream fixtures are intentionally byte-exact and retain upstream lints/tests.
#[rustfmt::skip]
#[allow(dead_code, clippy::all)]
#[path = "../fixtures/beta/common.rs"]
pub mod common;
#[path = "extensions/mod.rs"]
pub mod extensions;
