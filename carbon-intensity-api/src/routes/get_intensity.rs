use actix_web::{web, HttpResponse, Responder};

use crate::{
    application::AppState,
    entities::{Intensity, IntensityData},
};

pub async fn get_intensity(_data: web::Data<AppState>) -> std::io::Result<impl Responder> {
    let data = IntensityData {
        from: "2018-01-20T12:00Z".to_string(),
        to: "2018-01-20T12:30Z".to_string(),
        intensity: Intensity {
            forecast: 266,
            actual: 263,
            index: "moderate".to_string(),
        },
    };

    //TODO: invoke api

    Ok(HttpResponse::Ok().json(data))
}
