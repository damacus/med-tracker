pub mod auth;
pub mod backup;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod models;
pub mod portable;
pub mod resources;
pub mod sync;

pub use client::ApiClient;
pub use error::ApiError;
pub use models::{Capabilities, PortableImportMode};
pub use resources::ResourceKind;

pub const USER_AGENT: &str = concat!("medtracker-client-tools/", env!("CARGO_PKG_VERSION"));
