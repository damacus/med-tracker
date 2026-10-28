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

    async fn after_routes(&self, router: Router, ctx: &AppContext) -> Result<Router> {
        let canary_enabled = crate::models::canary::enabled(ctx);
        #[expect(
            clippy::result_large_err,
            reason = "Loco fixes this infallible callback to its framework error type"
        )]
        let view = TeraView::build_with_post_process(move |tera| {
            tera.global_context()
                .insert("canary_enabled", &canary_enabled);
            Ok(())
        })?;
        Ok(router.layer(Extension(ViewEngine::from(view))))
    }
}
