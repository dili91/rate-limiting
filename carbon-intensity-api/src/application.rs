use std::time::Duration;

use actix_web::{dev::Server, web, App, HttpServer};


use crate::{
    middleware::rate_limiter::ApiRateLimiterFactory, routes::get_intensity::get_intensity,
};

//TODO: add tracing

pub struct Application {
    http_server: Server,
    port: u16,

    //TODO: redis config
}

pub struct AppState {
    //TODO: pub carbon_intensity_client: ,
}

impl Application {
    /// Bakes the main HTTP server.
    pub fn build(host: &str, port: u16) -> Self {
        let app_state = web::Data::new(AppState {});

        let server = HttpServer::new(move || {
            App::new()
                // .wrap(
                //     // create cookie based session middleware
                //     SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                //         .cookie_secure(false)
                //         .build(),
                // )
                .app_data(app_state.clone())
                // .wrap_fn(|req, srv| {
                //     println!("Hi from start. You requested: {}", req.path());
                //     srv.call(req).map(|res| {
                //         println!("Hi from response");
                //         res
                //     })
                // })
                .wrap(ApiRateLimiterFactory::new(5, Duration::from_secs(60)))
                .route("/intensity", web::get().to(get_intensity))
        });

        //FIXME: improve
        let actix_server = server.bind((host, port)).expect("unable to build app");
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
