use std::time::Duration;

use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use rate_limiter_rs::{builders::RedisSettings, factory::RateLimiterFactory};

use crate::{
    middleware::rate_limiter::RateLimiterMiddlewareFactory,
    routes::{health_check::health_check, intensity::get_intensity::get_intensity},
    settings::AppSettings,
};

pub struct Application {
    http_server: Server,
    port: u16,
}

impl Application {
    /// Builds the main app entrypoint
    pub fn build(settings: AppSettings) -> Self {
        // let rate_limiter = RateLimiterFactory::fixed_token_bucket()
        //     .with_bucket_size(settings.rate_limiter.bucket_size)
        //     .with_bucket_validity(Duration::from_secs(
        //         settings.rate_limiter.bucket_validity_seconds,
        //     ))
        //     .with_redis_settings(RedisSettings {
        //         host: settings.rate_limiter.redis_server.host,
        //         port: settings.rate_limiter.redis_server.port,
        //     })
        //     .build()
        //     .expect("unable to setup rate limiter component");

            let rate_limiter = RateLimiterFactory::sliding_window()
            .with_window_size(settings.rate_limiter.bucket_size)
            .with_window_duration(Duration::from_secs(
                settings.rate_limiter.bucket_validity_seconds,
            ))
            .with_redis_settings(RedisSettings {
                host: settings.rate_limiter.redis_server.host,
                port: settings.rate_limiter.redis_server.port,
            })
            .build()
            .expect("unable to setup rate limiter component");    

        let server = HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .route("/health_check", web::get().to(health_check))
                .service(
                    web::scope("/carbon/intensity")
                        .wrap(RateLimiterMiddlewareFactory::with_rate_limiter(
                            rate_limiter.clone(),
                        ))
                        .route("", web::get().to(get_intensity)),
                )
        });

        let actix_server = server
            .bind((settings.http_server.host, settings.http_server.port))
            .expect("unable to build app");

        let port = actix_server.addrs()[0].port();
        let http_server = actix_server.run();
        Application { http_server, port }
    }

    /// List of addresses this server is bound to.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Actually starts running and accepting requests.
    pub fn run(self) -> Result<Server, std::io::Error> {
        Ok(self.http_server)
    }
}
