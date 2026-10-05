use async_trait::async_trait;
use axum::{Extension, Router};
use loco_rs::{
    Result,
    app::{AppContext, Initializer},
    controller::views::{ViewEngine, engines::TeraView},
};

pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> String {
        "view-engine".to_string()
    }

    async fn after_routes(&self, router: Router, _ctx: &AppContext) -> Result<Router> {
        Ok(router.layer(Extension(ViewEngine::from(TeraView::build()?))))
    }
}
