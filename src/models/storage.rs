use std::collections::BTreeMap;

use serde_json::Value;

pub(crate) const AVATAR_STORE: &str = "avatar";
pub(crate) const DMD_STORE: &str = "dmd";

#[derive(Clone)]
pub struct Operators {
    pub avatar: Option<opendal::Operator>,
    pub dmd: Option<opendal::Operator>,
}

pub fn configured(settings: Option<&Value>) -> Option<(loco_rs::storage::Storage, Operators)> {
    let avatar = avatar_operator(settings, None);
    let dmd = dmd_operator(settings, None);
    let mut stores: BTreeMap<String, Box<dyn loco_rs::storage::drivers::StoreDriver>> =
        BTreeMap::new();
    if let Some(operator) = &avatar {
        stores.insert(
            AVATAR_STORE.to_string(),
            Box::new(
                loco_rs::storage::drivers::opendal_adapter::OpendalAdapter::new(operator.clone()),
            ),
        );
    }
    if let Some(operator) = &dmd {
        stores.insert(
            DMD_STORE.to_string(),
            Box::new(
                loco_rs::storage::drivers::opendal_adapter::OpendalAdapter::new(operator.clone()),
            ),
        );
    }
    if stores.is_empty() {
        return None;
    }
    let primary = if stores.contains_key(AVATAR_STORE) {
        AVATAR_STORE
    } else {
        DMD_STORE
    };
    Some((
        loco_rs::storage::Storage::new(
            stores,
            Box::new(loco_rs::storage::strategies::single::SingleStrategy::new(
                primary,
            )),
        ),
        Operators { avatar, dmd },
    ))
}

pub(crate) fn avatar_operator(
    settings: Option<&Value>,
    bucket: Option<&str>,
) -> Option<opendal::Operator> {
    let section = "avatar_storage";
    let text =
        |key: &str| crate::models::settings::text(settings, &[section, key]).map(str::to_owned);
    let endpoint = text("endpoint")?;
    let configured_bucket = text("bucket")?;
    let region = text("region")?;
    let access_key = text("access_key_id")?;
    let secret_key = text("secret_access_key")?;
    let path_style =
        crate::models::settings::flag(settings, &[section, "force_path_style"]).unwrap_or(true);
    Some(s3_operator(
        &endpoint,
        bucket.unwrap_or(&configured_bucket),
        &region,
        &access_key,
        &secret_key,
        path_style,
    ))
}

fn dmd_operator(settings: Option<&Value>, bucket: Option<&str>) -> Option<opendal::Operator> {
    let section = "dmd_storage";
    let text =
        |key: &str| crate::models::settings::text(settings, &[section, key]).map(str::to_owned);
    let endpoint = text("endpoint")?;
    let configured_bucket = text("bucket")?;
    let access_key = text("access_key_id")?;
    let secret_key = text("secret_access_key")?;
    let region = text("region").unwrap_or_else(|| "us-east-1".to_string());
    Some(s3_operator(
        &endpoint,
        bucket.unwrap_or(&configured_bucket),
        &region,
        &access_key,
        &secret_key,
        true,
    ))
}

fn s3_operator(
    endpoint: &str,
    bucket: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    path_style: bool,
) -> opendal::Operator {
    let mut builder = opendal::services::S3::default()
        .bucket(bucket)
        .endpoint(endpoint)
        .region(region)
        .access_key_id(access_key)
        .secret_access_key(secret_key);
    if !path_style {
        builder = builder.enable_virtual_host_style();
    }
    let transport = opendal::HttpTransporter::new(
        opendal_http_transport_reqwest::ReqwestTransport::new(reqwest::Client::new()),
    );
    opendal::Operator::new(builder)
        .expect("S3 operator requires a bucket")
        .with_context(opendal::OperationContext::new().with_http_transport(transport))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::extract::{ConnectInfo, State};
    use axum::http::{Response, Uri};
    use bytes::Bytes;
    use serde_json::json;
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    struct RuntimeReadFixture {
        peers: mpsc::Sender<(String, SocketAddr)>,
        body_started: mpsc::Sender<()>,
        release_body: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
    }

    async fn runtime_read_response(
        State(fixture): State<Arc<RuntimeReadFixture>>,
        ConnectInfo(peer): ConnectInfo<SocketAddr>,
        uri: Uri,
    ) -> Response<Body> {
        let path = uri.path().to_string();
        fixture.peers.send((path.clone(), peer)).unwrap();
        if path.ends_with("/warm") {
            return Response::builder()
                .header("content-length", "4")
                .body(Body::from("warm"))
                .unwrap();
        }
        let gate = fixture.release_body.lock().unwrap().take().unwrap();
        let started = fixture.body_started.clone();
        let stream = futures_util::stream::once(async move {
            started.send(()).unwrap();
            let _ = gate.await;
            Ok::<Bytes, Infallible>(Bytes::from_static(b"held"))
        });
        Response::builder()
            .header("content-length", "4")
            .body(Body::from_stream(stream))
            .unwrap()
    }

    #[test]
    fn s3_read_survives_first_runtime_shutdown_during_reused_response() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (peer_tx, peer_rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let fixture = Arc::new(RuntimeReadFixture {
            peers: peer_tx,
            body_started: started_tx,
            release_body: Mutex::new(Some(release_rx)),
        });
        let (stop_server_tx, stop_server_rx) = tokio::sync::oneshot::channel::<()>();
        let server = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let routes = Router::new()
                    .fallback(runtime_read_response)
                    .with_state(fixture);
                tokio::select! {
                    result = axum::serve(listener, routes.into_make_service_with_connect_info::<SocketAddr>()) => result.unwrap(),
                    _ = stop_server_rx => {}
                }
            });
        });
        let (first_done_tx, first_done_rx) = mpsc::channel();
        let (stop_first_tx, stop_first_rx) = tokio::sync::oneshot::channel::<()>();
        let first_endpoint = endpoint.clone();
        let first = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let operator = s3_operator(
                    &first_endpoint,
                    "fixture",
                    "us-east-1",
                    "key",
                    "secret",
                    true,
                );
                let result = tokio::time::timeout(Duration::from_secs(25), operator.read("warm"))
                    .await
                    .map_err(|_| "warm read timed out".to_string())
                    .and_then(|result| {
                        result.map(|bytes| bytes.to_bytes()).map_err(|error| {
                            format!("kind={:?} temporary={}", error.kind(), error.is_temporary())
                        })
                    });
                first_done_tx.send(result).unwrap();
                let _ = stop_first_rx.await;
            });
        });
        let first_result = first_done_rx.recv_timeout(Duration::from_secs(30));
        let (second_done_tx, second_done_rx) = mpsc::channel();
        let second = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = runtime.block_on(async move {
                let operator =
                    s3_operator(&endpoint, "fixture", "us-east-1", "key", "secret", true);
                tokio::time::timeout(Duration::from_secs(25), operator.read("held"))
                    .await
                    .map_err(|_| "held read timed out".to_string())
                    .and_then(|result| {
                        result.map(|bytes| bytes.to_bytes()).map_err(|error| {
                            format!("kind={:?} temporary={}", error.kind(), error.is_temporary())
                        })
                    })
            });
            second_done_tx.send(result).unwrap();
        });
        let started = started_rx.recv_timeout(Duration::from_secs(30));
        let first_peer = peer_rx.recv_timeout(Duration::from_secs(30));
        let second_peer = peer_rx.recv_timeout(Duration::from_secs(30));
        let _ = stop_first_tx.send(());
        let first_join = first.join();
        let _ = release_tx.send(());
        let second_result = second_done_rx.recv_timeout(Duration::from_secs(30));
        let _ = stop_server_tx.send(());
        let server_join = server.join();
        let second_join = second.join();
        first_join.unwrap();
        server_join.unwrap();
        second_join.unwrap();
        let first_result = first_result.unwrap();
        let first_peer = first_peer.unwrap();
        let second_peer = second_peer.unwrap();
        let second_result = second_result.unwrap();
        assert_eq!(first_result.unwrap().as_ref(), b"warm");
        assert!(started.is_ok(), "second response body did not start");
        assert_eq!(first_peer.0, "/fixture/warm");
        assert_eq!(second_peer.0, "/fixture/held");
        if let Err(error) = &second_result {
            assert_eq!(
                first_peer.1, second_peer.1,
                "second read failed on a different socket; cause is inconclusive: {error}"
            );
        }
        assert_eq!(second_result.unwrap().as_ref(), b"held");
    }

    fn avatar_settings() -> Value {
        json!({
            "avatar_storage": {
                "endpoint": "http://127.0.0.1:9000",
                "bucket": "avatars",
                "region": "us-east-1",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        })
    }

    #[test]
    fn unconfigured_settings_produce_no_stores() {
        assert!(configured(None).is_none());
        assert!(configured(Some(&json!({}))).is_none());
    }

    #[test]
    fn fully_configured_avatar_storage_registers_named_store() {
        let (storage, operators) = configured(Some(&avatar_settings())).unwrap();
        assert!(storage.as_store(AVATAR_STORE).is_some());
        assert!(storage.as_store(DMD_STORE).is_none());
        assert!(operators.avatar.is_some());
        assert!(operators.dmd.is_none());
    }

    #[test]
    fn partially_configured_storage_is_skipped() {
        let mut settings = avatar_settings();
        settings["avatar_storage"]["bucket"] = Value::Null;
        assert!(configured(Some(&settings)).is_none());
    }

    #[test]
    fn dmd_storage_defaults_region_and_registers() {
        let settings = json!({
            "dmd_storage": {
                "endpoint": "http://127.0.0.1:9000",
                "bucket": "dmd",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        });
        let (storage, operators) = configured(Some(&settings)).unwrap();
        assert!(storage.as_store(DMD_STORE).is_some());
        assert!(operators.dmd.is_some());
    }

    #[test]
    fn empty_rendered_values_count_as_unconfigured() {
        let settings = json!({
            "avatar_storage": {
                "endpoint": "",
                "bucket": "avatars",
                "region": "us-east-1",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        });
        assert!(configured(Some(&settings)).is_none());
    }
}
