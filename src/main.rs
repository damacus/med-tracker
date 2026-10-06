use med_tracker::app::App;
use migration::Migrator;

#[tokio::main]
#[allow(
    clippy::result_large_err,
    reason = "Loco's framework error type contains ordered JSON values"
)]
async fn main() -> loco_rs::Result<()> {
    loco_rs::cli::main::<App, Migrator>().await
}
