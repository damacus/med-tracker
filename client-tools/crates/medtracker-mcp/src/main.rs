mod server;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .without_time()
        .init();

    if std::env::args().any(|argument| argument == "--schema") {
        println!("{}", serde_json::to_string_pretty(&tools::tool_list())?);
        return Ok(());
    }

    server::run_stdio().await
}
