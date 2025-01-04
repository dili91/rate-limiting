use actix_web::{HttpRequest, HttpResponse, Responder};

use super::entities::{CarbonIntensityData, Intensity, IntensityIndex};

pub async fn get_intensity(req: HttpRequest) -> std::io::Result<impl Responder> {
    println!("{:?}", req.headers());

    // return just mock data for now
    let data = CarbonIntensityData {
        from: "2018-01-20T12:00Z".to_string(),
        to: "2018-01-20T12:30Z".to_string(),
        intensity: Intensity {
            forecast: 266,
            actual: 263,
            index: IntensityIndex::Moderate,
        },
    };

    Ok(HttpResponse::Ok().json(data))
}
