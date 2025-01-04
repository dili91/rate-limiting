use std::{rc::Rc, time::Duration};

use actix_web::{body::MessageBody, dev::{Server, ServiceRequest, ServiceResponse}, middleware::Logger, web, App, Error, HttpServer};
use rate_limiter_rs::{builders::RedisSettings, factory::RateLimiterFactory};
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder, TracingLogger};

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
        let rate_limiter = RateLimiterFactory::fixed_window()
            .with_window_size(settings.rate_limiter.window_size)
            .with_window_duration(Duration::from_secs(
                settings.rate_limiter.window_duration_seconds,
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
                .wrap(TracingLogger::default())
                .route("/health_check", web::get().to(health_check))
                .service(
                    web::scope("/carbon/intensity")
                        .wrap(RateLimiterMiddlewareFactory::with_rate_limiter(Rc::new(
                            rate_limiter.clone(),
                        )))
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
