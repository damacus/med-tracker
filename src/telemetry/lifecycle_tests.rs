use opentelemetry_sdk::{
    error::{OTelSdkError, OTelSdkResult},
    trace::{SdkTracerProvider, Span, SpanData, SpanProcessor},
};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct Processor {
    flushes: Arc<AtomicUsize>,
    shutdowns: Arc<AtomicUsize>,
    fail: bool,
    delay: Duration,
    shutdown_delay: Duration,
    gate: Option<Arc<(Mutex<bool>, Condvar)>>,
}

impl SpanProcessor for Processor {
    fn on_start(&self, _: &mut Span, _: &opentelemetry::Context) {}
    fn on_end(&self, _: SpanData) {}
    fn force_flush(&self) -> OTelSdkResult {
        self.flushes.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            let (lock, condition) = &**gate;
            let mut released = lock.lock().expect("gate");
            while !*released {
                released = condition.wait(released).expect("gate");
            }
        }
        std::thread::sleep(self.delay);
        if self.fail {
            Err(OTelSdkError::InternalFailure("synthetic".into()))
        } else {
            Ok(())
        }
    }
    fn shutdown_with_timeout(&self, _: Duration) -> OTelSdkResult {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.shutdown_delay);
        Ok(())
    }
}

fn install(
    fail: bool,
    delay: Duration,
    shutdown_delay: Duration,
    gate: Option<Arc<(Mutex<bool>, Condvar)>>,
) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let flushes = Arc::new(AtomicUsize::new(0));
    let shutdowns = Arc::new(AtomicUsize::new(0));
    let processor = Processor {
        flushes: flushes.clone(),
        shutdowns: shutdowns.clone(),
        fail,
        delay,
        shutdown_delay,
        gate,
    };
    let provider = SdkTracerProvider::builder()
        .with_span_processor(processor)
        .build();
    super::TRACER_PROVIDER.set(provider).expect("provider");
    (flushes, shutdowns)
}

fn process_exit_is_bounded(test_name: &str, shutdown: bool, stalled_shutdown: bool) {
    const MARKER: &str = "MEDTRACKER_LIFECYCLE_CHILD";
    if std::env::var_os(MARKER).is_some() {
        let flush_delay = if stalled_shutdown {
            Duration::ZERO
        } else {
            Duration::from_secs(4)
        };
        let shutdown_delay = if stalled_shutdown {
            Duration::from_secs(4)
        } else {
            Duration::ZERO
        };
        install(false, flush_delay, shutdown_delay, None);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime");
        if shutdown {
            runtime.block_on(super::shutdown());
        } else {
            runtime.block_on(super::flush());
        }
        drop(runtime);
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", test_name, "--nocapture"])
        .env(MARKER, "1")
        .spawn()
        .expect("test process");
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("wait for child") {
            assert!(status.success(), "child failed: {status}");
            break;
        }
        if started.elapsed() > Duration::from_millis(3500) {
            child.kill().expect("stop overdue test process");
            child.wait().expect("reap test process");
            panic!("runtime drop and process exit exceeded deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn flush_allows_process_exit_after_timeout() {
    process_exit_is_bounded(
        "telemetry::lifecycle_tests::flush_allows_process_exit_after_timeout",
        false,
        false,
    );
}

#[test]
fn shutdown_allows_process_exit_after_timeout() {
    process_exit_is_bounded(
        "telemetry::lifecycle_tests::shutdown_allows_process_exit_after_timeout",
        true,
        false,
    );
}

#[test]
fn stalled_shutdown_allows_process_exit_after_timeout() {
    process_exit_is_bounded(
        "telemetry::lifecycle_tests::stalled_shutdown_allows_process_exit_after_timeout",
        true,
        true,
    );
}

#[tokio::test]
async fn success_flush_and_shutdown_complete() {
    let (flushes, shutdowns) = install(false, Duration::ZERO, Duration::ZERO, None);
    super::flush().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 1);
    super::shutdown().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 2);
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failed_flush_does_not_prevent_shutdown() {
    let (flushes, shutdowns) = install(true, Duration::ZERO, Duration::ZERO, None);
    super::flush().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 1);
    super::shutdown().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 2);
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn overlapping_flushes_keep_one_outstanding_worker_and_honor_shutdown() {
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let (flushes, shutdowns) = install(false, Duration::ZERO, Duration::ZERO, Some(gate.clone()));
    tokio::join!(super::flush(), super::flush(), super::shutdown());
    for _ in 0..3 {
        assert!(
            tokio::time::timeout(Duration::from_millis(20), super::flush())
                .await
                .is_err()
        );
    }
    let count = flushes.load(Ordering::SeqCst);
    let (lock, condition) = &*gate;
    *lock.lock().expect("gate") = true;
    condition.notify_all();
    for _ in 0..100 {
        if shutdowns.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(count, 1);
    assert_eq!(flushes.load(Ordering::SeqCst), 2);
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}

#[derive(Debug)]
struct PanicOnce {
    flushes: Arc<AtomicUsize>,
    shutdowns: Arc<AtomicUsize>,
}

impl SpanProcessor for PanicOnce {
    fn on_start(&self, _: &mut Span, _: &opentelemetry::Context) {}
    fn on_end(&self, _: SpanData) {}
    fn force_flush(&self) -> OTelSdkResult {
        assert_ne!(
            self.flushes.fetch_add(1, Ordering::SeqCst),
            0,
            "synthetic processor panic"
        );
        Ok(())
    }
    fn shutdown_with_timeout(&self, _: Duration) -> OTelSdkResult {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn processor_panic_does_not_permanently_strand_lifecycle() {
    let flushes = Arc::new(AtomicUsize::new(0));
    let shutdowns = Arc::new(AtomicUsize::new(0));
    let provider = SdkTracerProvider::builder()
        .with_span_processor(PanicOnce {
            flushes: flushes.clone(),
            shutdowns: shutdowns.clone(),
        })
        .build();
    super::TRACER_PROVIDER.set(provider).expect("provider");
    super::flush().await;
    super::flush().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 2);
    super::shutdown().await;
    assert_eq!(flushes.load(Ordering::SeqCst), 3);
    assert_eq!(shutdowns.load(Ordering::SeqCst), 1);
}
