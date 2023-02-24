use wasm_bindgen::prelude::*;

mod client;
mod multi_party_ecdsa;
pub mod utils;

// Required for rayon thread support
pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    if wasm_log::try_init(wasm_log::Config::new(log::Level::Debug)).is_ok() {
        log::info!("WASM logger initialized");
    }
    log::info!("WASM: module started {:?}", std::thread::current().id());
}
