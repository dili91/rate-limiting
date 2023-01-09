use std::{
    future::{ready, Ready},
    net::IpAddr,
    rc::Rc,
    str::FromStr,
    time::Duration,
};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{
        header::{HeaderName, HeaderValue},
        StatusCode,
    },
    Error as ActixWebError, HttpResponse,
};

use derive_more::Display;
use futures_util::{future::LocalBoxFuture, FutureExt};
use rate_limiter_rs::{builder::{RateLimiterBuilder, RedisSettings}, entities::TokenBucketRateLimiter, RateLimiter};

pub struct ApiRateLimiterFactory {
    rate_limiter: Rc<TokenBucketRateLimiter>,
}

//TODO: get rid of unwraps on this file
impl ApiRateLimiterFactory {
    pub fn new(bucket_size: usize, bucket_validity: Duration) -> ApiRateLimiterFactory {
        let rate_limiter = RateLimiterBuilder::default()
            .with_bucket_size(bucket_size)
            .with_bucket_validity(bucket_validity)
            .with_redis_settings(RedisSettings{ host: "redis".to_string(), port: 6379 })
            .build()
            .unwrap();

        ApiRateLimiterFactory {
            rate_limiter: Rc::new(rate_limiter),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiRateLimiterFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixWebError> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixWebError;
    type InitError = ();
    type Transform = ApiRateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiRateLimiterMiddleware {
            service: Rc::new(service),
            rate_limiter: self.rate_limiter.clone(),
        }))
    }
}

pub struct ApiRateLimiterMiddleware<S> {
    service: Rc<S>,
    rate_limiter: Rc<TokenBucketRateLimiter>,
}

impl<S, B> Service<ServiceRequest> for ApiRateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixWebError> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixWebError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let rate_limiter = self.rate_limiter.clone();
        async move {
            //TODO: gracefully handle unwraps
            let ip_address: IpAddr = req
                .connection_info()
                .realip_remote_addr()
                .unwrap()
                .parse()
                .unwrap();

            let rate_limiter_response = rate_limiter.is_request_allowed(ip_address).unwrap();

            if !rate_limiter_response.is_request_allowed {
                eprintln!("Request throttled for ip {}", ip_address);
                return Err(MyError {
                    retry_after_seconds: rate_limiter_response.expire_in.as_secs(),
                }
                .into());
            }

            let mut res = service.call(req).await?;

            //FIXME
            res.headers_mut().append(
                HeaderName::from_str("X-Rate-Limiter-Tokens").unwrap(),
                HeaderValue::from_str(rate_limiter_response.remaining_request_counter.to_string().as_str())
                    .unwrap(),
            );

            Ok(res)
        }
        .boxed_local()
    }
}

#[derive(Debug, Display)]
pub struct MyError {
    pub retry_after_seconds: u64,
}

impl actix_web::error::ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(("X-Rate-Limiter-Retry-After", self.retry_after_seconds))
            .finish()
    }

    fn status_code(&self) -> StatusCode {
        // match *self {
        //     MyError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        //     MyError::BadClientData => StatusCode::BAD_REQUEST,
        //     MyError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        // }
        StatusCode::TOO_MANY_REQUESTS
    }
}
