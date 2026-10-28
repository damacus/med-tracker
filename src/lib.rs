pub mod app;
pub mod controllers;
pub mod initializers;
pub mod models;
mod request_trace;
pub mod views;

#[cfg(test)]
#[path = "../tests/support/template.rs"]
mod template;
