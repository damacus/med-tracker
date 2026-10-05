use med_tracker::app::App;
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    loco_rs::cli::main::<App, Migrator>().await
}
