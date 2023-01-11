use std::fmt::Display;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CarbonIntensityData {
    pub from: String,
    pub to: String,
    pub intensity: Intensity,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Intensity {
    pub forecast: u32,
    pub actual: u32,
    pub index: IntensityIndex,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum IntensityIndex {
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

impl Display for IntensityIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntensityIndex::Moderate => write!(f, "moderate"),
            IntensityIndex::VeryLow => write!(f, "very low"),
            IntensityIndex::Low => write!(f, "low"),
            IntensityIndex::High => write!(f, "high"),
            IntensityIndex::VeryHigh => write!(f, "very high"),
        }
    }
}
