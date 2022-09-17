#[macro_export]
macro_rules! get_env {
    ($key:literal) => {
        std::env::var($key)
            .map_err(|_| anyhow::anyhow!("Environment variable {} is not set.", $key))?
    };
}
