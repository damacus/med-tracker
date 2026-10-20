use std::future::Future;
use std::io::BufRead;
use std::panic::AssertUnwindSafe;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::FutureExt;
use playwright_rs::protocol::{
    APIRequestContext, Animations, AriaRole, Browser, BrowserContext, BrowserContextOptions,
    GetByRoleOptions, Page, ScreenshotOptions, Viewport,
};
use playwright_rs::{Playwright, expect};
use sea_orm::DatabaseConnection;

use crate::db::OwnedDatabase;
use crate::{fixtures, template};

/// `Buffer.alloc(64, 7).toString('base64')` — the same fixed 64-byte test key
/// the old JS harness generated.
const SESSION_KEY: &str =
    "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBw==";

/// Browsers install once per process, but playwright-rs protocol channels
/// bind to the launching tokio runtime — each `#[tokio::test]` gets its own, so
/// every Journey launches a fresh Playwright + Browser rather than sharing one.
static INSTALLED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn launch_browser() -> (Playwright, Browser) {
    INSTALLED
        .get_or_init(|| async {
            playwright_rs::install_browsers(Some(&["chromium"]))
                .await
                .unwrap();
        })
        .await;
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    (playwright, browser)
}

/// Dropping the guard kills the server if `close` has not already reaped it.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(unix)]
#[test]
fn dropping_server_guard_reaps_child() {
    let child = Command::new("sleep")
        .arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    drop(KillOnDrop(child));
    let process = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "stat="])
        .output()
        .unwrap();
    assert!(
        process.stdout.is_empty(),
        "server process {pid} remains after dropping its guard: {}",
        String::from_utf8_lossy(&process.stdout)
    );
}

pub struct Journey {
    pub page: Page,
    context: BrowserContext,
    pub admin: DatabaseConnection,
    pub origin: String,
    db: OwnedDatabase,
    server: KillOnDrop,
    log: Arc<Mutex<String>>,
    // Keeps the driver connection alive; the browser borrows its runtime.
    _playwright: Playwright,
}

impl Journey {
    async fn start(viewport: Viewport) -> Self {
        let uri = template::owned_database_url();
        template::prepare(&uri, "browser_template", |admin| async move {
            fixtures::seed_core(&admin).await;
            fixtures::seed_oauth_clients(&admin).await;
            fixtures::seed_browser_rows(&admin).await;
        })
        .await;
        let db =
            OwnedDatabase::clone(&uri, "browser_template", "browser_", "browser_runtime_").await;
        let booted = AssertUnwindSafe(Self::boot(&db.runtime_uri, viewport))
            .catch_unwind()
            .await;
        let (server, log, playwright, context, page, origin) = match booted {
            Ok(parts) => parts,
            Err(payload) => {
                db.close().await;
                std::panic::resume_unwind(payload);
            }
        };
        Self {
            page,
            context,
            admin: db.admin.clone(),
            origin,
            db,
            server,
            log,
            _playwright: playwright,
        }
    }

    async fn boot(
        runtime_uri: &str,
        viewport: Viewport,
    ) -> (
        KillOnDrop,
        Arc<Mutex<String>>,
        Playwright,
        BrowserContext,
        Page,
        String,
    ) {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let origin = format!("http://localhost:{port}");
        let mut command = Command::new(env!("CARGO_BIN_EXE_med-tracker"));
        command
            .args(["start", "--environment", "test"])
            .env("TZ", "UTC")
            .env("BINDING", "127.0.0.1")
            .env("PORT", port.to_string())
            .env("MEDTRACKER_PUBLIC_HOST", "http://localhost")
            .env("DATABASE_URL", runtime_uri)
            .env("MEDTRACKER_SESSION_KEY", SESSION_KEY)
            .env("MEDTRACKER_COOKIE_SECURE", "false")
            .env("MEDTRACKER_CAPTURE_MAIL", "false")
            .env("MEDTRACKER_SMTP_PORT", "1025")
            .env(
                "RAILS_SECRET_KEY_BASE",
                "synthetic-rails-secret-key-base-for-compatibility",
            )
            .env(
                "RAILS_OLD_SECRET_KEY_BASE",
                "synthetic-old-rails-secret-key-base",
            )
            .env(
                "AUTH_SESSION_SECRET",
                "synthetic-browser-occurrence-signing-key-32",
            )
            .env_remove("RUST_LOG")
            .env_remove("INVITE_ONLY")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, _) in std::env::vars_os() {
            let key = key.to_string_lossy().into_owned();
            if key.starts_with("MEDTRACKER_ZITADEL_") || key.starts_with("MEDTRACKER_WEB_PUSH_") {
                command.env_remove(&key);
            }
        }
        let mut server = KillOnDrop(command.spawn().expect("spawn med-tracker server"));
        let log = Arc::new(Mutex::new(String::new()));
        let streams: Vec<Box<dyn std::io::Read + Send>> = vec![
            Box::new(server.0.stdout.take().unwrap()),
            Box::new(server.0.stderr.take().unwrap()),
        ];
        for stream in streams {
            let log = Arc::clone(&log);
            std::thread::spawn(move || {
                let mut reader = std::io::BufReader::new(stream);
                let mut line = String::new();
                while reader.read_line(&mut line).unwrap_or(0) > 0 {
                    let mut guard = log.lock().unwrap();
                    guard.push_str(&line);
                    if guard.len() > 65536 {
                        let mut cut = guard.len() - 65536;
                        while !guard.is_char_boundary(cut) {
                            cut += 1;
                        }
                        guard.drain(..cut);
                    }
                    line.clear();
                }
            });
        }
        let (playwright, browser) = launch_browser().await;
        let context = browser
            .new_context_with_options(
                BrowserContextOptions::builder()
                    .viewport(viewport)
                    .locale("en-GB".to_string())
                    .build(),
            )
            .await
            .unwrap();
        let page = context.new_page().await.unwrap();
        let request = context.request().await.unwrap();
        wait_for_server(&request, &origin, &mut server, &log).await;
        for (asset, content_type) in [
            ("/static/css/compiled.css", "text/css"),
            ("/static/js/html5-qrcode.min.js", "javascript"),
        ] {
            let response = request
                .get(&format!("{origin}{asset}"), None)
                .await
                .unwrap();
            assert!(response.ok(), "browser asset unavailable: {asset}");
            assert!(
                response
                    .headers()
                    .get("content-type")
                    .is_some_and(|value| value.contains(content_type)),
                "browser asset returned unexpected content: {asset}"
            );
        }
        (server, log, playwright, context, page, origin)
    }

    pub fn log_tail(&self) -> String {
        self.log.lock().unwrap().clone()
    }

    async fn close(mut self) {
        let _ = self.server.0.kill();
        let _ = self.server.0.wait();
        let _ = self.context.close().await;
        self.db.close().await;
    }
}

async fn wait_for_server(
    request: &APIRequestContext,
    origin: &str,
    server: &mut KillOnDrop,
    log: &Arc<Mutex<String>>,
) {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(response) = request.get(&format!("{origin}/up"), None).await
            && response.ok()
        {
            return;
        }
        if let Some(status) = server.0.try_wait().unwrap() {
            panic!(
                "server exited early ({status}):
{}",
                log.lock().unwrap()
            );
        }
        assert!(
            Instant::now() < deadline,
            "server did not become ready:
{}",
            log.lock().unwrap()
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub async fn journey<F>(viewport: Viewport, body: F)
where
    F: FnOnce(&mut Journey) -> std::pin::Pin<Box<dyn Future<Output = ()> + '_>>,
{
    let mut journey = Journey::start(viewport).await;
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        AssertUnwindSafe(body(&mut journey)).catch_unwind(),
    )
    .await;
    match result {
        Ok(Ok(())) => journey.close().await,
        Ok(Err(payload)) => {
            eprintln!(
                "--- server log tail ---
{}",
                journey.log_tail()
            );
            journey.close().await;
            std::panic::resume_unwind(payload);
        }
        Err(_) => {
            let tail = journey.log_tail();
            journey.close().await;
            panic!(
                "journey timed out after 60s
--- server log tail ---
{tail}"
            );
        }
    }
}

pub fn named(name: &str) -> Option<GetByRoleOptions> {
    Some(GetByRoleOptions::default().name(name).exact(true))
}

pub async fn sign_in(journey: &Journey) {
    let page = &journey.page;
    page.goto(&format!("{}/login", journey.origin), None)
        .await
        .unwrap();
    page.get_by_label("Email address", true)
        .fill("persistence@example.test", None)
        .await
        .unwrap();
    page.get_by_label("Password", true)
        .fill("password", None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Sign in"))
        .click(None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Button, named("Open Synthetic household"))
        .click(None)
        .await
        .unwrap();
    page.goto(
        &format!(
            "{}/households/persistence-fixture/medications",
            journey.origin
        ),
        None,
    )
    .await
    .unwrap();
    expect(page.locator("header [data-app-drawer]"))
        .to_be_visible()
        .await
        .unwrap();
}

/// Port of tests/browser/care-navigation.mjs: reach a named section from
/// whichever navigation chrome is present (sidebar, dashboard menu, or the
/// account/navigation dialogs).
pub async fn open_care_section(page: &Page, name: &str) {
    let dashboard = page.locator(".dashboard-sidebar");
    if dashboard.count().await.unwrap() > 0 {
        if dashboard.is_visible().await.unwrap() {
            dashboard
                .get_by_role(AriaRole::Link, named(name))
                .click(None)
                .await
                .unwrap();
        } else {
            page.get_by_role(AriaRole::Button, named("Open menu"))
                .click(None)
                .await
                .unwrap();
            page.locator("#dashboard-menu")
                .get_by_role(AriaRole::Link, named(name))
                .click(None)
                .await
                .unwrap();
        }
        return;
    }
    let account = ["Administration", "Profile"].contains(&name);
    let button = if account { "My Account" } else { "Open menu" };
    let dialog_name = if account {
        "My Account"
    } else {
        "Navigation menu"
    };
    page.get_by_role(AriaRole::Button, named(button))
        .click(None)
        .await
        .unwrap();
    page.get_by_role(AriaRole::Dialog, named(dialog_name))
        .get_by_role(AriaRole::Link, named(name))
        .click(None)
        .await
        .unwrap();
}

pub async fn capture(page: &Page, name: &str) {
    let directory = std::path::Path::new("docs/screenshots/browser-journeys");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .build(),
    )
    .await
    .unwrap();
}
