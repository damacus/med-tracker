use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

pub(super) async fn household_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/household.css"),
    )
        .into_response()
}

pub(super) async fn dashboard_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/dashboard.css"),
    )
        .into_response()
}

pub(super) async fn leptodon_styles() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../ui-preview/public/preview.css"),
    )
        .into_response()
}

pub(super) async fn dashboard_hydrate_script() -> Response {
    let script = include_str!("../../../web/src/assets/dashboard-hydrate.js")
        .replace(
            "__PKG_VERSION__",
            &format!(
                "{:016x}",
                medtracker_web::dashboard::DASHBOARD_HYDRATE_PKG_VERSION
            ),
        )
        .replace(
            "__WASM_VERSION__",
            &format!(
                "{:016x}",
                *medtracker_web::dashboard::DASHBOARD_HYDRATE_WASM_VERSION
            ),
        );
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        script,
    )
        .into_response()
}

pub(super) async fn dashboard_hydrate_package() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../../ui-preview/public/pkg/medtracker_ui_preview.js"),
    )
        .into_response()
}

pub(super) async fn dashboard_hydrate_wasm() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/wasm"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_bytes!("../../../ui-preview/public/pkg/medtracker_ui_preview_bg.wasm").as_slice(),
    )
        .into_response()
}

pub(super) async fn dashboard_script() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/assets/dashboard-register.js"),
    )
        .into_response()
}

pub(super) async fn dashboard_worker() -> Response {
    let worker = include_str!("../../../web/src/assets/dashboard-sw.js")
        .replace(
            "__CSS_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_CSS_VERSION),
        )
        .replace(
            "__JS_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_JS_VERSION),
        )
        .replace(
            "__SW_VERSION__",
            &format!("{:016x}", medtracker_web::dashboard::DASHBOARD_SW_VERSION),
        );
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        worker,
    )
        .into_response()
}

pub(super) async fn dashboard_manifest() -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/assets/dashboard-manifest.webmanifest"),
    )
        .into_response()
}

pub(super) async fn dashboard_offline() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("../../../web/src/assets/dashboard-offline.html"),
    )
        .into_response()
}

pub(super) async fn dashboard_reconnect() -> Response {
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, "/login"),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

pub(super) async fn dashboard_icon_192() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/dashboard-icon-192.png").as_slice(),
    )
        .into_response()
}

pub(super) async fn dashboard_icon_512() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/dashboard-icon-512.png").as_slice(),
    )
        .into_response()
}

pub(super) async fn inter_regular() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/inter-v20-latin-regular.woff2").as_slice(),
    )
        .into_response()
}

pub(super) async fn inter_500() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/inter-v20-latin-500.woff2").as_slice(),
    )
        .into_response()
}

pub(super) async fn inter_800() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/inter-v20-latin-800.woff2").as_slice(),
    )
        .into_response()
}

pub(super) async fn inter_600() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/inter-v20-latin-600.woff2").as_slice(),
    )
        .into_response()
}

pub(super) async fn inter_700() -> Response {
    (
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("../../../web/src/assets/inter-v20-latin-700.woff2").as_slice(),
    )
        .into_response()
}

pub(super) async fn styles() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        medtracker_web::medication_stylesheet(),
    )
        .into_response()
}

pub(super) async fn script() -> Response {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        medtracker_web::medication_script(),
    )
        .into_response()
}
