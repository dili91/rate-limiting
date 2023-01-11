use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub http_server: ServerSettings,
    pub rate_limiter: RateLimiterSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimiterSettings {
    pub bucket_size: u64,
    pub bucket_validity_seconds: u64,
    pub redis_server: ServerSettings,
}

const DEFAULT_HTTP_SERVER_HOST: &str = "0.0.0.0";
const DEFAULT_HTTP_SERVER_PORT: u16 = 9000;
const DEFAULT_RATE_LIMITER_BUCKET_SIZE: u64 = 5;
const DEFAULT_RATE_LIMITER_BUCKET_VALIDITY_SECONDS: u64 = 60;
const DEFAULT_REDIS_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_REDIS_SERVER_PORT: u16 = 6379;

impl AppSettings {
    pub fn new() -> Result<Self, ConfigError> {
        let config_builder = Config::builder()
            .set_default("http_server.host", DEFAULT_HTTP_SERVER_HOST)?
            .set_default("http_server.port", DEFAULT_HTTP_SERVER_PORT)?
            .set_default("rate_limiter.bucket_size", DEFAULT_RATE_LIMITER_BUCKET_SIZE)?
            .set_default(
                "rate_limiter.bucket_validity_seconds",
                DEFAULT_RATE_LIMITER_BUCKET_VALIDITY_SECONDS,
            )?
            .set_default("rate_limiter.redis_server.host", DEFAULT_REDIS_SERVER_HOST)?
            .set_default("rate_limiter.redis_server.port", DEFAULT_REDIS_SERVER_PORT)?
            .add_source(Environment::default())
            .add_source(
                Environment::with_prefix("app")
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()?;

        config_builder.try_deserialize()
    }
}
