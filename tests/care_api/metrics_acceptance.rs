use super::*;
use axum::{body::Bytes, routing::post};
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::common::v1::any_value;
use opentelemetry_proto::tonic::metrics::v1::{metric, number_data_point};
use prost::Message;
use sea_orm::{
    TransactionTrait,
    sea_query::{Alias, Expr, ExprTrait, Func, Query},
};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn configured_metrics_export_real_request_without_private_values() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
    let collector = axum::Router::new().route(
        "/v1/metrics",
        post(move |body: Bytes| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(body);
                axum::http::StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, collector).await.unwrap() });
    let endpoint = format!("http://{address}/v1/metrics");
    let app = Application::new_configured(None, |settings| {
        settings["telemetry"] = json!({"otlp_metrics_endpoint": endpoint});
    })
    .await;
    assert!(med_tracker::app::App::init_logger(&app.context).unwrap());
    let response = app
        .client
        .request(
            reqwest::Method::from_bytes(b"PRIVATE_METHOD_SENTINEL").unwrap(),
            format!(
                "{}/metrics-private-sentinel?token=query-secret-sentinel",
                app.origin
            ),
        )
        .header("authorization", "header-secret-sentinel")
        .body("body-secret-sentinel")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    let matched = app
        .client
        .post(format!(
            "{}/api/v1/households/987654321/schedules?token=matched-query-sentinel",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let matched_status = matched.status().as_u16();
    med_tracker::telemetry::flush().await;
    let payload = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("configured metrics must export on flush")
        .expect("collector must remain open");
    let bytes = payload.as_ref();
    assert!(
        bytes
            .windows(b"http.server.request.duration".len())
            .any(|w| w == b"http.server.request.duration")
    );
    assert!(
        !bytes
            .windows(b"metrics-private-sentinel".len())
            .any(|w| w == b"metrics-private-sentinel")
    );
    assert!(
        !bytes
            .windows(b"query-secret-sentinel".len())
            .any(|w| w == b"query-secret-sentinel")
    );
    assert!(
        !bytes
            .windows(b"header-secret-sentinel".len())
            .any(|w| w == b"header-secret-sentinel")
    );
    assert!(
        !bytes
            .windows(b"body-secret-sentinel".len())
            .any(|w| w == b"body-secret-sentinel")
    );
    assert!(
        !bytes
            .windows(b"PRIVATE_METHOD_SENTINEL".len())
            .any(|w| w == b"PRIVATE_METHOD_SENTINEL")
    );
    assert!(bytes.windows(b"_OTHER".len()).any(|w| w == b"_OTHER"));
    assert!(
        !bytes
            .windows(b"matched-query-sentinel".len())
            .any(|w| w == b"matched-query-sentinel")
    );
    assert!(!bytes.windows(b"987654321".len()).any(|w| w == b"987654321"));
    let exported = ExportMetricsServiceRequest::decode(bytes).unwrap();
    let matched_point = exported.resource_metrics.iter().flat_map(|resource| &resource.scope_metrics)
        .flat_map(|scope| &scope.metrics)
        .filter(|metric| metric.name == "http.server.request.duration" && metric.unit == "s")
        .filter_map(|metric| match &metric.data { Some(metric::Data::Histogram(histogram)) => Some(histogram), _ => None })
        .flat_map(|histogram| &histogram.data_points)
        .find(|point| point.attributes.iter().any(|attr| attr.key == "http.route"
            && matches!(attr.value.as_ref().and_then(|v| v.value.as_ref()),
                Some(any_value::Value::StringValue(value)) if value == "/api/v1/households/{household_id}/schedules")))
        .expect("matched route template histogram");
    assert!(matched_point.count > 0 && matched_point.sum.is_some_and(|sum| sum >= 0.0));
    assert!(matched_point.attributes.iter().any(|attr| attr.key == "http.request.method"
        && matches!(attr.value.as_ref().and_then(|v| v.value.as_ref()), Some(any_value::Value::StringValue(value)) if value == "POST")));
    assert!(matched_point.attributes.iter().any(|attr| attr.key == "http.response.status_code"
        && matches!(attr.value.as_ref().and_then(|v| v.value.as_ref()), Some(any_value::Value::IntValue(value)) if *value == i64::from(matched_status))));
    app.close().await;
    server.abort();
}

fn gauge(payload: &[u8], name: &str, label: Option<&str>) -> Option<i64> {
    let request = ExportMetricsServiceRequest::decode(payload).ok()?;
    for resource in request.resource_metrics {
        for scope in resource.scope_metrics {
            for metric in scope.metrics {
                if metric.name != name {
                    continue;
                }
                if let Some(metric::Data::Gauge(gauge)) = metric.data {
                    for point in gauge.data_points {
                        let matches = label.is_none_or(|label| {
                            point.attributes.iter().any(|attr| {
                                matches!(attr.value.as_ref().and_then(|value| value.value.as_ref()),
                                Some(any_value::Value::StringValue(value)) if value == label)
                            })
                        });
                        if matches && let Some(number_data_point::Value::AsInt(value)) = point.value
                        {
                            return Some(value);
                        }
                    }
                }
            }
        }
    }
    None
}

#[tokio::test(flavor = "multi_thread")]
async fn trace_base_endpoint_does_not_enable_metric_export() {
    let (trace_tx, mut traces) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
    let (metric_tx, mut metrics) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
    let collector = axum::Router::new()
        .route(
            "/v1/traces",
            post(move |body: Bytes| {
                let tx = trace_tx.clone();
                async move {
                    let _ = tx.send(body);
                    axum::http::StatusCode::OK
                }
            }),
        )
        .route(
            "/v1/metrics",
            post(move |body: Bytes| {
                let tx = metric_tx.clone();
                async move {
                    let _ = tx.send(body);
                    axum::http::StatusCode::OK
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, collector).await.unwrap() });
    let app = Application::new_configured(None, |settings| {
        settings["telemetry"] = json!({"otlp_endpoint": format!("http://{address}")});
    })
    .await;
    assert!(med_tracker::app::App::init_logger(&app.context).unwrap());
    let _ = app
        .client
        .get(format!("{}/", app.origin))
        .send()
        .await
        .unwrap();
    drop(tracing::warn_span!("synthetic trace endpoint check"));
    med_tracker::telemetry::flush().await;
    assert!(
        tokio::time::timeout(Duration::from_secs(2), traces.recv())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(250), metrics.recv())
            .await
            .is_err()
    );
    app.close().await;
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn unavailable_metrics_endpoint_does_not_fail_request() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let app = Application::new_configured(None, |settings| {
        settings["telemetry"] = json!({
            "otlp_metrics_endpoint": format!("http://{address}/v1/metrics"),
            "otlp_timeout_ms": 100
        });
    })
    .await;
    assert!(med_tracker::app::App::init_logger(&app.context).unwrap());
    let response = app
        .client
        .get(format!("{}/missing", app.origin))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
    tokio::time::timeout(Duration::from_secs(2), med_tracker::telemetry::flush())
        .await
        .unwrap();
    app.close().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn worker_queue_and_running_pool_export_cached_values_without_waiting_for_pool() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Bytes>();
    let collector = axum::Router::new().route(
        "/v1/metrics",
        post(move |body: Bytes| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(body);
                axum::http::StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, collector).await.unwrap() });
    let endpoint = format!("http://{address}/v1/metrics");
    let initial = Application::new_configured(None, |settings| {
        settings["telemetry"] = json!({"otlp_metrics_endpoint": endpoint});
    })
    .await;
    assert!(med_tracker::app::App::init_logger(&initial.context).unwrap());
    let now = chrono::Utc::now();
    let db = initial.fixture.admin.clone();
    let states = [
        "queued",
        "processing",
        "completed",
        "failed",
        "cancelled",
        "other",
    ];
    let baseline_query = Query::select()
        .column(Alias::new("status"))
        .expr_as(Expr::col(Alias::new("status")).count(), Alias::new("total"))
        .from(Alias::new("pg_loco_queue"))
        .group_by_col(Alias::new("status"))
        .to_owned();
    let mut baseline = [0_i64; 6];
    for row in db
        .query_all_raw(db.get_database_backend().build(&baseline_query))
        .await
        .unwrap()
    {
        let state: String = row.try_get("", "status").unwrap();
        let count: i64 = row.try_get("", "total").unwrap();
        let index = states[..5]
            .iter()
            .position(|name| *name == state)
            .unwrap_or(5);
        baseline[index] += count;
    }
    let oldest_query = Query::select()
        .expr_as(
            Func::min(Expr::col(Alias::new("run_at"))),
            Alias::new("oldest"),
        )
        .from(Alias::new("pg_loco_queue"))
        .and_where(Expr::col(Alias::new("status")).eq("queued"))
        .and_where(Expr::col(Alias::new("run_at")).lte(now))
        .to_owned();
    let oldest: Option<chrono::DateTime<chrono::Utc>> = db
        .query_one_raw(db.get_database_backend().build(&oldest_query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "oldest")
        .unwrap();
    let expected_oldest = oldest.map_or(now - chrono::Duration::seconds(90), |oldest| {
        std::cmp::min(oldest, now - chrono::Duration::seconds(90))
    });
    let ids: Vec<String> = (0..3).map(|_| uuid::Uuid::new_v4().to_string()).collect();
    let job_ids = ids.clone();
    let mut config = initial.context.config.clone();
    config.database.min_connections = 1;
    config.database.max_connections = 1;
    let outcome = tokio::spawn(async move {
        for ((state, run_at), id) in [
            ("queued", now - chrono::Duration::seconds(90)),
            ("queued", now + chrono::Duration::hours(2)),
            ("future_vendor_state", now),
        ]
        .into_iter()
        .zip(job_ids)
        {
            let query = Query::insert()
                .into_table(Alias::new("pg_loco_queue"))
                .columns(["id", "name", "task_data", "status", "run_at"].map(Alias::new))
                .values_panic([
                    id.into(),
                    "synthetic".into(),
                    json!({}).into(),
                    state.into(),
                    run_at.into(),
                ])
                .to_owned();
            db.execute_raw(db.get_database_backend().build(&query))
                .await
                .unwrap();
        }
        let boot = med_tracker::app::App::boot(
            StartMode::ServerAndWorker,
            &loco_rs::environment::Environment::Test,
            config,
        )
        .await
        .unwrap();
        let mut found = None;
        for _ in 0..40 {
            med_tracker::telemetry::flush().await;
            let payload = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .unwrap()
                .unwrap();
            if gauge(&payload, "medtracker.queue.jobs", Some("queued")) == Some(baseline[0] + 2) {
                found = Some(payload);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let payload =
            found.expect("worker refresh must publish the ready and future queue entries");
        for (index, state) in states.iter().enumerate() {
            let added = if index == 0 {
                2
            } else if index == 5 {
                1
            } else {
                0
            };
            assert_eq!(
                gauge(&payload, "medtracker.queue.jobs", Some(state)),
                Some(baseline[index] + added)
            );
        }
        let age = gauge(&payload, "medtracker.queue.oldest_queued_age_s", None).unwrap();
        let expected_age = chrono::Utc::now()
            .signed_duration_since(expected_oldest)
            .num_seconds();
        assert!(
            (expected_age - 3..=expected_age + 2).contains(&age),
            "due job age: {age}"
        );
        assert_eq!(gauge(&payload, "db.client.connection.max", None), Some(1));
        let transaction = boot.app_context.db.begin().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), med_tracker::telemetry::flush())
            .await
            .expect("observing a busy pool must not block");
        let payload = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            gauge(&payload, "db.client.connection.count", Some("used")),
            Some(1)
        );
        transaction.rollback().await.unwrap();
        med_tracker::app::App::on_shutdown(&boot.app_context).await;
        drop(boot);
    })
    .await;
    let cleanup = Query::delete()
        .from_table(Alias::new("pg_loco_queue"))
        .and_where(Expr::col(Alias::new("id")).is_in(ids))
        .to_owned();
    initial
        .fixture
        .admin
        .execute_raw(initial.fixture.admin.get_database_backend().build(&cleanup))
        .await
        .unwrap();
    initial.close().await;
    server.abort();
    outcome.expect("worker metrics acceptance must pass");
}
