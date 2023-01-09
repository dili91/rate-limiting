// Sample data:
//{
//     "from": "2018-01-20T12:00Z",
//     "to": "2018-01-20T12:30Z",
//     "intensity": {
//         "forecast": 266,
//         "actual": 263,
//         "index": "moderate"
//     }
// }

#[derive(serde::Serialize)]
pub struct IntensityData {
    pub from: String,
    pub to: String,
    pub intensity: Intensity,
}

#[derive(serde::Serialize)]
pub struct Intensity {
    pub forecast: u32,
    pub actual: u32,
    //FIXME: enum with possible states:
    pub index: String,
}
