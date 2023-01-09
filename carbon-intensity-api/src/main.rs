use carbon_intensity_api::application::Application;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    //TODO: review bind address
    let app = Application::build("0.0.0.0", 9000);

    println!("Carbon intensity API starting on port {:?}", app.port());

    app.run()?.await
}
