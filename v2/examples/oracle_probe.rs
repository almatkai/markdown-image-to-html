//! Test-only JSON probe, using the production adapter and exact beta CoreContext.
include!("../tests/support/tauri_stub.rs");
#[path = "../tests/support/host.rs"]
mod native_core;
#[path = "../tests/support/scenario.rs"]
mod scenario;
use serde_json::Value;
use std::io::Read;
fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&input).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let results = runtime.block_on(async {
        let mut results = Vec::new();
        for case in cases {
            results.push(scenario::execute(&case).await);
        }
        results
    });
    println!("{}", serde_json::to_string(&results).unwrap());
}
