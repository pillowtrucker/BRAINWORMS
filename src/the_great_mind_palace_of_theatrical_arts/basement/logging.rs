pub fn register_logger() {
    #[cfg(not(target_os = "android"))]
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        //.filter_module("rend3", log::LevelFilter::Info)
        .parse_default_env()
        .init();
}
