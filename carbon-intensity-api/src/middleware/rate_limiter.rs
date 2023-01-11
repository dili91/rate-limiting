use std::{
    future::{ready, Ready},
    net::AddrParseError,
    rc::Rc,
    str::FromStr,
};

use actix_web::http::header::{InvalidHeaderName, InvalidHeaderValue};

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
use rate_limiter_rs::{
    entities::{
        RateLimiterResponse, RequestAllowed, RequestIdentifier, RequestThrottled,
        TokenBucketRateLimiter,
    },
    RateLimiter,
};

pub const RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME: &str = "X-Remaining-Request";
pub const RATE_LIMITER_RETRY_AFTER_HTTP_HEADER_NAME: &str = "Retry-After";

pub struct RateLimiterMiddlewareFactory {
    rate_limiter: Rc<TokenBucketRateLimiter>,
}

impl RateLimiterMiddlewareFactory {
    pub fn with_rate_limiter(rate_limiter: TokenBucketRateLimiter) -> RateLimiterMiddlewareFactory {
        RateLimiterMiddlewareFactory {
            rate_limiter: Rc::new(rate_limiter),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiterMiddlewareFactory
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
            let ip_address = req
                .connection_info()
                .realip_remote_addr()
                .ok_or_else(|| ApiError::InvalidRequest("Missing IP address!".to_string()))?
                .parse()
                .map_err(|e: AddrParseError| ApiError::Internal(e.to_string()))?;

            let request_identifier = RequestIdentifier::Ip(ip_address);

            let rate_limiter_response = rate_limiter.check_request(request_identifier);

            return match rate_limiter_response {
                Ok(response) => {
                    return match response {
                        RateLimiterResponse::RequestAllowed(RequestAllowed {
                            remaining_request_counter,
                        }) => {
                            let mut inner_service_response = service.call(req).await?;

                            inner_service_response.headers_mut().insert(
                                HeaderName::from_str(
                                    RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME,
                                )
                                .map_err(
                                    |e: InvalidHeaderName| ApiError::Internal(e.to_string()),
                                )?,
                                HeaderValue::from_str(
                                    remaining_request_counter.to_string().as_str(),
                                )
                                .map_err(
                                    |e: InvalidHeaderValue| ApiError::Internal(e.to_string()),
                                )?,
                            );

                            Ok(inner_service_response)
                        }
                        RateLimiterResponse::RequestThrottled(RequestThrottled { retry_in }) => {
                            log::warn!("request throttled for ip={}", ip_address);

                            return Err(ApiError::RequestThrottled {
                                retry_after_seconds: retry_in.as_secs(),
                            }
                            .into());
                        }
                    };
                }
                Err(_err) => {
                    log::warn!("unable to check rate limit for request coming from ip={}. Skipping validation", ip_address);
                    Ok(service.call(req).await?)
                }
            };
        }
        .boxed_local()
    }
}

#[derive(Debug, Display)]
pub enum ApiError {
    RequestThrottled { retry_after_seconds: u64 },
    InvalidRequest(String),
    Internal(String),
}

impl actix_web::error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::RequestThrottled {
                retry_after_seconds,
            } => HttpResponse::build(self.status_code())
                .insert_header((
                    RATE_LIMITER_RETRY_AFTER_HTTP_HEADER_NAME,
                    retry_after_seconds.to_string(),
                ))
                .body("You've been throttled!"),
            _ => HttpResponse::build(self.status_code()).finish(),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::RequestThrottled { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
