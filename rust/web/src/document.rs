pub(super) fn document(title: &str, body: String) -> String {
    let title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"theme-color\" content=\"#f3f8fb\"><link rel=\"stylesheet\" href=\"/auth.css\"><script defer src=\"/profile.js\"></script><title>{title} | MedTracker</title></head><body>{body}</body></html>"
    )
}

pub(super) fn authenticated_document(title: &str, csrf: &str, body: String) -> String {
    document(title, body).replacen(
        "<head>",
        &format!("<head><meta name=\"csrf-token\" content=\"{csrf}\">"),
        1,
    )
}

pub(super) fn medication_document(title: &str, csrf: &str, body: String) -> String {
    authenticated_document(title, csrf, body)
        .replacen("</head>", "<link rel=\"stylesheet\" href=\"/medication.css\"><script defer src=\"/medication.js\"></script></head>", 1)
}
