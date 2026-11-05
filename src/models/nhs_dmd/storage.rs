use crate::models::errors::OperationError;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::StreamExt;
use loco_rs::app::AppContext;
use md5::{Digest, Md5};
use std::time::Duration;

pub const MAX_ARCHIVE_BYTES: usize = 500 * 1024 * 1024;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ArchiveStore {
    operator: opendal::Operator,
}

impl ArchiveStore {
    pub fn configured(ctx: &AppContext) -> Result<Self, OperationError> {
        ctx.storage
            .as_store(crate::models::storage::DMD_STORE)
            .ok_or_else(|| OperationError::unavailable("storage"))?;
        let operator = ctx
            .shared_store
            .get::<crate::models::storage::Operators>()
            .and_then(|operators| operators.dmd)
            .ok_or_else(|| OperationError::unavailable("storage"))?;
        Ok(Self { operator })
    }

    pub async fn put(&self, key: &str, bytes: Vec<u8>) -> Result<(), OperationError> {
        use tracing::Instrument;
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            self.operator
                .write_with(key, bytes)
                .content_type("application/zip")
                .into_future()
                .instrument(crate::telemetry::units::storage_span("put")),
        )
        .await
        .map_err(OperationError::from)?
        .map(|_| ())
        .map_err(OperationError::from)
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, OperationError> {
        use tracing::Instrument;
        let span = tracing::info_span!(
            "storage",
            otel.name = "storage get",
            otel.kind = "client",
            medtracker.outcome = tracing::field::Empty,
            error.type = tracing::field::Empty,
        );
        let result = async {
            let reader =
                tokio::time::timeout(OPERATION_TIMEOUT, self.operator.reader(key).into_future())
                    .await
                    .map_err(OperationError::from)?
                    .map_err(OperationError::from)?;
            let mut stream = tokio::time::timeout(OPERATION_TIMEOUT, reader.into_bytes_stream(..))
                .await
                .map_err(OperationError::from)?
                .map_err(OperationError::from)?;
            let mut bytes = Vec::new();
            let collect = async {
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(OperationError::from)?;
                    if bytes.len().saturating_add(chunk.len()) > MAX_ARCHIVE_BYTES {
                        return Err(OperationError::unavailable("storage"));
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(())
            };
            tokio::time::timeout(OPERATION_TIMEOUT, collect)
                .await
                .map_err(OperationError::from)??;
            Ok(bytes)
        }
        .instrument(span.clone())
        .await;
        match &result {
            Ok(_) => {
                span.record("medtracker.outcome", "success");
            }
            Err(_) => {
                span.record("medtracker.outcome", "failure");
                span.record("error.type", "unavailable");
            }
        };
        result
    }

    pub async fn delete(&self, key: &str) -> Result<(), OperationError> {
        use tracing::Instrument;
        tokio::time::timeout(
            OPERATION_TIMEOUT,
            self.operator
                .delete(key)
                .into_future()
                .instrument(crate::telemetry::units::storage_span("delete")),
        )
        .await
        .map_err(OperationError::from)?
        .map_err(OperationError::from)
    }
}

pub fn checksum(bytes: &[u8]) -> String {
    STANDARD.encode(Md5::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::ArchiveStore;
    use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::prelude::*;

    async fn read_from_server(
        truncated: bool,
    ) -> (
        Result<Vec<u8>, crate::models::errors::OperationError>,
        Vec<opentelemetry_sdk::trace::SpanData>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let mut request = [0; 4096];
                let count = socket.read(&mut request).await.unwrap();
                let head = request[..count].starts_with(b"HEAD ");
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n")
                    .await
                    .unwrap();
                if !head {
                    socket
                        .write_all(if truncated { &b"he"[..] } else { &b"hello"[..] })
                        .await
                        .unwrap();
                    break;
                }
            }
        });
        let builder = opendal::services::S3::default()
            .bucket("test")
            .endpoint(&format!("http://{address}"))
            .region("us-east-1")
            .access_key_id("test")
            .secret_access_key("test");
        let transport = opendal::HttpTransporter::new(
            opendal_http_transport_reqwest::ReqwestTransport::new(reqwest::Client::new()),
        );
        let store = ArchiveStore {
            operator: opendal::Operator::new(builder)
                .unwrap()
                .with_context(opendal::OperationContext::new().with_http_transport(transport)),
        };
        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
            .with_simple_exporter(exporter.clone())
            .build();
        let subscriber = tracing_subscriber::registry()
            .with(crate::telemetry::otel_layer(&provider, "storage-test"));
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            store.get("archive.zip").with_subscriber(subscriber),
        )
        .await
        .expect("storage read completed");
        server.abort();
        provider.force_flush().unwrap();
        (result, exporter.get_finished_spans().unwrap())
    }

    #[tokio::test]
    async fn get_span_records_success_after_read() {
        let (result, spans) = read_from_server(false).await;
        assert_eq!(result.unwrap(), b"hello");
        let span = spans
            .iter()
            .find(|span| span.name == "storage get")
            .unwrap();
        assert!(
            span.attributes
                .iter()
                .any(|kv| kv.key.as_str() == "medtracker.outcome"
                    && kv.value.to_string() == "success")
        );
    }

    #[tokio::test]
    async fn get_span_records_stream_failure() {
        let (result, spans) = read_from_server(true).await;
        assert!(result.is_err());
        let span = spans
            .iter()
            .find(|span| span.name == "storage get")
            .unwrap();
        assert!(
            span.attributes
                .iter()
                .any(|kv| kv.key.as_str() == "medtracker.outcome"
                    && kv.value.to_string() == "failure")
        );
        assert!(
            span.attributes
                .iter()
                .any(|kv| kv.key.as_str() == "error.type" && kv.value.to_string() == "unavailable")
        );
    }
}
