use std::{env, net::SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = env::var("WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:38175".to_owned())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, medtracker_web::app()).await?;
    Ok(())
}
