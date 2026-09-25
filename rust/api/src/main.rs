#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let state = medtracker_api::connect(&database_url)
        .await
        .expect("connect to PostgreSQL");
    let address = std::env::var("API_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:39998".to_owned());
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind API listener");
    axum::serve(listener, medtracker_api::router(state))
        .await
        .expect("serve API");
}
