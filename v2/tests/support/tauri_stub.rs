// Only the Tauri surface needed by byte-exact common.rs; never opens an app.
extern crate self as tauri;
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: serde_json::Value) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: serde_json::Value) -> Result<(), String> {
        panic!("Tauri must never be called by this no-view adapter")
    }
}
