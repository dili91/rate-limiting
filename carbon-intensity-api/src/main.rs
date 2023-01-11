use carbon_intensity_api::{application::Application, settings::AppSettings};
use env_logger::Env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let settings = AppSettings::new().expect("unable to load app settings");

    let app = Application::build(settings);

    log::info!("Carbon intensity API starting on port {}...", app.port());

    app.run()?.await
}
