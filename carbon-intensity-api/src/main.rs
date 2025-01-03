use std::sync::LazyLock;

use carbon_intensity_api::{application::Application, settings::AppSettings};
use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_sdk::{runtime::TokioCurrentThread, Resource};
use opentelemetry_semantic_conventions::resource;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const APP_NAME: &str = "carbon-intensity-api";
static RESOURCE: LazyLock<Resource> =
    LazyLock::new(|| Resource::new(vec![KeyValue::new(resource::SERVICE_NAME, APP_NAME)]));

fn init_telemetry() {
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to build the span exporter");

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, TokioCurrentThread)
        .with_resource(RESOURCE.clone())
        .build();
    let tracer = provider.tracer(APP_NAME);

    // Filter based on level - trace, debug, info, warn, error
    // Tunable via `RUST_LOG` env variable
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));

    // Create a `tracing` layer using the otlp tracer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create a `tracing` layer to emit spans as structured logs to stdout
    let formatting_layer = BunyanFormattingLayer::new(APP_NAME.into(), std::io::stdout);

    // Combined them all together in a `tracing` subscriber
    let subscriber = Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to install `tracing` subscriber.")
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_telemetry();

    let settings = AppSettings::new().expect("unable to load app settings");

    let app = Application::build(settings);

    log::info!("Carbon intensity API starting on port {}...", app.port());

    app.run()?.await
}
