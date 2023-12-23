pub fn register_logger() {
    #[cfg(target_arch = "wasm32")]
    console_log::init().unwrap();

    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    env_logger::builder()
        .filter_module("rend3", log::LevelFilter::Info)
        .parse_default_env()
        .init();
}
pub fn register_panic_hook() {
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
