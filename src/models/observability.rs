use async_trait::async_trait;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use opentelemetry::{KeyValue, global, trace::Tracer};
use uuid::Uuid;

use crate::models::errors::OperationError;

const CANARY_KINDS: [&str; 2] = ["application_event", "job"];

pub async fn emit_canary(ctx: &AppContext) -> Result<(), OperationError> {
    let workflow_id = Uuid::new_v4();
    for kind in CANARY_KINDS {
        emit(ctx, kind, workflow_id);
    }
    Ok(())
}

fn emit(ctx: &AppContext, kind: &str, workflow_id: Uuid) {
    let event_id = Uuid::new_v4();
    let attempt_id = Uuid::new_v4();
    global::tracer("medtracker.observability_canary").in_span("observability.canary", |_| {
        tracing::info!(
            event.name = "observability.canary",
            event.outcome = "success",
            event.dataset = "medtracker.application",
            event.id = %event_id,
            medtracker.schema.version = 1_i64,
            medtracker.reason = "canary_emitted",
            medtracker.canary.kind = kind,
            workflow.id = %workflow_id,
            attempt.id = %attempt_id,
            service.name = "medtracker",
            service.version = env!("CARGO_PKG_VERSION"),
            service.environment = %ctx.environment,
        );
        global::meter("medtracker.observability_canary")
            .u64_counter("medtracker.observability.canary")
            .with_unit("1")
            .with_description("Safe deployed observability canary")
            .build()
            .add(1, &[KeyValue::new("canary.kind", kind.to_owned())]);
    });
}

pub struct ObservabilityCanaryTask;

#[async_trait]
impl Task for ObservabilityCanaryTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "observability-canary".into(),
            detail: "Emit the safe deployed observability canary".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> loco_rs::Result<()> {
        emit_canary(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Observability canary failed"))
    }
}
