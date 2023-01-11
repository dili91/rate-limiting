use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};


use carbon_intensity_api::{
    application::Application,
    middleware::rate_limiter::{
        RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME, RATE_LIMITER_RETRY_AFTER_HTTP_HEADER_NAME,
    },
    routes::intensity::entities::CarbonIntensityData,
    settings::{AppSettings, RateLimiterSettings, ServerSettings},
};
use rand::Rng;

use reqwest::{
    header::{HeaderName, HeaderValue},
    Client, Response,
};

#[tokio::test]
async fn should_return_200_if_within_request_limit() {
    //arrange
    let rate_limiter_settings = RateLimiterSettings {
        bucket_size: 5,
        bucket_validity_seconds: 15,
        redis_server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 6379,
        },
    };
    let api_endpoint = spawn_app(rate_limiter_settings);
    let x_forwarded_for_ip_address = generate_random_ip();

    //act
    let response = get_intensity_data(api_endpoint, x_forwarded_for_ip_address).await;

    //assert
    assert!(response.status().is_success());
    assert!(response
        .headers()
        .contains_key(RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME));
    let res: CarbonIntensityData = response
        .json()
        .await
        .expect("unable to deserialize init_registration response into JSON object");
    assert_eq!(res.from, "2018-01-20T12:00Z");
    assert_eq!(res.to, "2018-01-20T12:30Z");
    assert_eq!(res.intensity.index.to_string(), "moderate");
    assert_eq!(res.intensity.actual, 263);
    assert_eq!(res.intensity.forecast, 266);
}

#[tokio::test]
async fn should_return_200_if_unable_to_check_rate_limit() {
    //arrange
    let rate_limiter_settings = RateLimiterSettings {
        bucket_size: 5,
        bucket_validity_seconds: 15,
        redis_server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 6378,
        },
    };
    let api_endpoint = spawn_app(rate_limiter_settings);
    let x_forwarded_for_ip_address = generate_random_ip();

    //act
    let response = get_intensity_data(api_endpoint.clone(), x_forwarded_for_ip_address).await;

    //assert
    assert!(response.status().is_success());
    assert!(!response
        .headers()
        .contains_key(RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME));
}

#[tokio::test]
async fn should_return_429_if_request_is_throttled() {
    //arrange
    let rate_limiter_settings = RateLimiterSettings {
        bucket_size: 5,
        bucket_validity_seconds: 15,
        redis_server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 6379,
        },
    };
    let api_endpoint = spawn_app(rate_limiter_settings.clone());
    let x_forwarded_for_ip_address = generate_random_ip();

    //act & assert
    for _i in 0..rate_limiter_settings.bucket_size {
        let response = get_intensity_data(api_endpoint.clone(), x_forwarded_for_ip_address).await;
        assert!(response.status().is_success());
        assert!(response
            .headers()
            .contains_key(RATE_LIMITER_REMAINING_REQUEST_HTTP_HEADER_NAME));
    }

    let throttled = get_intensity_data(api_endpoint.clone(), x_forwarded_for_ip_address).await;
    assert_eq!(throttled.status().as_u16(), 429);
    assert!(throttled
        .headers()
        .contains_key(RATE_LIMITER_RETRY_AFTER_HTTP_HEADER_NAME));
}

#[tokio::test]
async fn should_never_return_429_on_non_rate_limited_endpoints() {
    //arrange
    let rate_limiter_settings = RateLimiterSettings {
        bucket_size: 5,
        bucket_validity_seconds: 15,
        redis_server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 6379,
        },
    };
    let api_endpoint = spawn_app(rate_limiter_settings.clone());
    let x_forwarded_for_ip_address = generate_random_ip();

    //act & assert
    for _i in 0..3 * rate_limiter_settings.bucket_size {
        // We're firing requests for 3 times limit configured on the rate limiter
        let response = get_test_client()
            .get(format!("{api_endpoint}/health_check"))
            .header(
                HeaderName::from_str("X-Forwarded-For").unwrap(),
                HeaderValue::from_str(&x_forwarded_for_ip_address.to_string()).unwrap(),
            )
            .send()
            .await
            .expect("failed to query health_check endpoint.");

        assert!(response.status().is_success());
    }
}

fn spawn_app(rate_limiter_settings: RateLimiterSettings) -> String {
    let app_settings = AppSettings {
        http_server: ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 0,
        },
        rate_limiter: rate_limiter_settings,
    };

    let app = Application::build(app_settings);
    let port = app.port();
    let server = app.run().expect("failed to bind test app");
    //spawn the app server as a background task.
    //the handle returned by Tokio is currently not used
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

fn get_test_client() -> Client {
    reqwest::Client::new()
}

fn generate_random_ip() -> IpAddr {
    let mut rng = rand::thread_rng();
    IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
}

async fn get_intensity_data(api_endpoint: String, ip_address: IpAddr) -> Response {
    get_test_client()
        .get(format!("{api_endpoint}/carbon/intensity"))
        .header(
            HeaderName::from_str("X-Forwarded-For").unwrap(),
            HeaderValue::from_str(&ip_address.to_string()).unwrap(),
        )
        .send()
        .await
        .expect("failed to get carbon intensity data")
}
