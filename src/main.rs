mod process;
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("extension runtime");
    let result = runtime.block_on(tezbar_extension_sdk::serve(process::Adapter));
    runtime.shutdown_timeout(std::time::Duration::from_millis(500));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
